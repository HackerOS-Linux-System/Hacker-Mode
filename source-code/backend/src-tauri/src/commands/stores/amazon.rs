use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
pub struct AmazonProvider;

/// `nile auth --login` blokuje do czasu zakończenia logowania przez
/// użytkownika w przeglądarce. `login_start` (wymagane przez trait
/// `StoreProvider`) musi zwrócić się szybko z samym URL-em, więc proces
/// jest "odkładany" tutaj, a jego zakończenia pilnuje osobny wątek
/// uruchamiany z `store_login::open_login_window` (tam, gdzie mamy
/// `AppHandle` do wyemitowania eventu z wynikiem) — patrz `take_pending_child`.
static PENDING_LOGIN_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

/// Zabiera odłożony proces logowania (jeśli jakiś czeka) — wywoływane
/// dokładnie raz po `login_start`, z wątku, który go doczeka i zgłosi
/// wynik. Zwraca `None`, jeśli nie ma żadnego oczekującego procesu (np.
/// `login_start` nie został jeszcze wywołany albo już go ktoś odebrał).
pub fn take_pending_login_child() -> Option<Child> {
    PENDING_LOGIN_CHILD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .take()
}

#[derive(Debug, Deserialize)]
struct NileInstalledGame {
    id: String,
    #[serde(alias = "title")]
    product_title: Option<String>,
    path: Option<String>,
}

impl StoreProvider for AmazonProvider {
    fn platform(&self) -> Platform {
        Platform::Amazon
    }

    fn is_available(&self) -> bool {
        tool_available("nile")
    }

    fn is_logged_in(&self) -> bool {
        Command::new("nile")
            .args(["library", "list"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        let mut child = Command::new("nile")
            .args(["auth", "--login"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| StoreError::ExecFailed("nile".into(), e.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| StoreError::ExecFailed("nile".into(), "brak stdout".into()))?;
        let reader = BufReader::new(stdout);

        // `nile auth --login` sam prowadzi cały flow logowania i (wg jego
        // dokumentacji) blokuje do czasu zakończenia przez użytkownika w
        // przeglądarce — celowo NIE czekamy na zakończenie procesu tutaj
        // (`child.wait()`), tylko wyciągamy pierwszy URL, jaki wypisze, i
        // pozwalamy procesowi kontynuować w tle. `is_logged_in` odpytywane
        // z UI po zamknięciu okna logowania wykryje sukces.
        let mut found_url = None;
        for line in reader.lines().take(20).flatten() {
            if let Some(start) = line.find("https://") {
                found_url = Some(line[start..].trim().to_string());
                break;
            }
        }

        match found_url {
            Some(url) => {
                // Odkładamy proces do doczekania przez `store_login.rs` —
                // NIE porzucamy go bez śledzenia wyniku (to był błąd w
                // poprzedniej wersji: proces stawał się sierotą, a UI nie
                // dostawało żadnej informacji o niepowodzeniu logowania).
                *PENDING_LOGIN_CHILD.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(child);
                Ok(LoginFlow {
                    url: Some(url),
                    instructions: "Zaloguj się na swoje konto Amazon w otwartym oknie. Po \
                        zakończeniu po prostu zamknij to okno — Hacker Mode sprawdzi, czy \
                        logowanie się powiodło."
                        .to_string(),
                    needs_code: false,
                })
            }
            None => Err(StoreError::ParseError(
                "nile auth --login".into(),
                "nie znaleziono URL-a logowania na wyjściu `nile` — sprawdź ręcznie w \
                 terminalu poleceniem `nile auth --login`"
                    .into(),
            )),
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("nile")
            .args(["library", "list", "--installed", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("nile".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("Amazon Games".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Trzymamy też surowe wartości JSON obok typowanych `NileInstalledGame`
        // (patrz `find_cover_art` niżej) — w przeciwieństwie do Epic/GOG,
        // `nile` nie ma udokumentowanego, stabilnego formatu cache'u
        // metadanych ani publicznego API katalogowego, którego dałoby się
        // tu wprost odpytać. Zamiast zgadywać nieistniejący endpoint,
        // przepuszczamy KAŻDY wpis z `nile library list ... --json`
        // przez tę samą heurystykę co GOG (`cover_cache::find_image_url`)
        // — jeśli `nile` kiedykolwiek zacznie zwracać pole z URL-em
        // okładki (np. `productImage`/`thumbnail`), zostanie ono
        // automatycznie wykryte i pobrane, bez konieczności zmiany kodu
        // pod konkretną nazwę pola.
        let raw_entries: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("nile library list".into(), e.to_string()))?;

        let parsed: Vec<NileInstalledGame> = raw_entries
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(|e: serde_json::Error| StoreError::ParseError("nile library list".into(), e.to_string()))?;

        let ids: Vec<String> = parsed.iter().map(|e| e.id.clone()).collect();
        let cover_urls: Vec<Option<String>> = raw_entries.iter().map(crate::cover_cache::find_image_url).collect();
        let covers = fetch_covers_in_parallel(&ids, &cover_urls);

        let games = parsed
            .into_iter()
            .zip(covers)
            .map(|(entry, cover_path)| Game {
                title: entry.product_title.clone().unwrap_or_else(|| format!("Gra Amazon #{}", entry.id)),
                id: entry.id,
                platform: Platform::Amazon,
                installed: true,
                install_dir: entry.path,
                cover_path,
                playtime_minutes: None,
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["install", game_id]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["uninstall", game_id]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["launch", game_id]);
        Ok(cmd)
    }
}

/// Pobiera (o ile `url` nie jest `None` — patrz wywołanie w `list_games`)
/// okładkę spod `url` i zapisuje ją w lokalnym cache Hacker Mode.
/// Przyjmuje współdzielony `client`, żeby dało się to wywoływać
/// równolegle — patrz `fetch_covers_in_parallel`.
fn find_cover_art(id: &str, url: Option<&str>, client: &reqwest::blocking::Client) -> Option<String> {
    let url = url?;
    // Tak jak w GOG, adresy bywają zapisane bez schematu (`//cdn.../x.jpg`).
    let url = if let Some(stripped) = url.strip_prefix("//") {
        format!("https://{stripped}")
    } else {
        url.to_string()
    };
    crate::cover_cache::cache_image(&url, &format!("amazon-{id}"), client)
}

/// Pobiera okładki dla wielu gier Amazon naraz na ograniczonej puli
/// wątków — patrz `gog.rs::fetch_covers_in_parallel` po pełny opis
/// strategii. Różnica względem GOG/Epic: tu URL okładki (jeśli w ogóle
/// istnieje) jest już znany z góry (wyciągnięty heurystycznie z surowego
/// JSON-a `nile`, patrz `list_games`), więc wątki tylko pobierają i
/// cache'ują obraz, bez dodatkowego zapytania sieciowego po metadane.
fn fetch_covers_in_parallel(ids: &[String], cover_urls: &[Option<String>]) -> Vec<Option<String>> {
    const MAX_PARALLEL_COVER_FETCHES: usize = 8;

    let mut covers: Vec<Option<String>> = vec![None; ids.len()];
    if ids.is_empty() || cover_urls.iter().all(Option::is_none) {
        // Żaden wpis nie ma URL-a okładki (typowy dziś przypadek dla
        // Amazon, patrz komentarz w `list_games`) — nie ma sensu nawet
        // budować klienta HTTP ani odpalać wątków.
        return covers;
    }

    let Some(client) = crate::cover_cache::build_client() else {
        return covers;
    };

    let worker_count = MAX_PARALLEL_COVER_FETCHES.min(ids.len());
    let chunk_size = (ids.len() + worker_count - 1) / worker_count;

    std::thread::scope(|scope| {
        let id_chunks = ids.chunks(chunk_size);
        let url_chunks = cover_urls.chunks(chunk_size);
        let cover_chunks = covers.chunks_mut(chunk_size);
        for ((ids_chunk, urls_chunk), covers_chunk) in id_chunks.zip(url_chunks).zip(cover_chunks) {
            let client = &client;
            scope.spawn(move || {
                for ((id, url), slot) in ids_chunk.iter().zip(urls_chunk.iter()).zip(covers_chunk.iter_mut()) {
                    *slot = find_cover_art(id, url.as_deref(), client);
                }
            });
        }
    });

    covers
}

/// Odpowiednik `steam::fetch_owned_games` dla Amazon: pełna biblioteka
/// POSIADANYCH gier na koncie (nie tylko zainstalowanych) — różnica
/// względem `list_games` to tylko brak flagi `--installed` w wywołaniu
/// `nile`, którą społeczność `nile`/Heroic dokumentuje jako pokazującą
/// pełną bibliotekę Amazon Games/Prime Gaming, gdy się ją pominie
/// (`nile library list` bez żadnych flag). Ciche błędy — patrz
/// `epic::fetch_owned_games`, ten sam powód.
pub fn fetch_owned_games(_installed_ids: &std::collections::HashSet<String>) -> Vec<super::OwnedGame> {
    let output = Command::new("nile")
        .args(["library", "list", "--json"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw_entries: Vec<serde_json::Value> = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(err) => {
            tracing::debug!(%err, "Amazon: nie udało się sparsować `nile library list --json`");
            return Vec::new();
        }
    };

    let parsed: Vec<NileInstalledGame> = raw_entries
        .iter()
        .cloned()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let ids: Vec<String> = parsed.iter().map(|e| e.id.clone()).collect();
    let cover_urls: Vec<Option<String>> = raw_entries.iter().map(crate::cover_cache::find_image_url).collect();
    let covers = fetch_covers_in_parallel(&ids, &cover_urls);

    parsed
        .into_iter()
        .zip(covers)
        .map(|(entry, cover_path)| super::OwnedGame {
            title: entry.product_title.clone().unwrap_or_else(|| format!("Gra Amazon #{}", entry.id)),
            id: entry.id,
            cover_path,
        })
        .collect()
}
