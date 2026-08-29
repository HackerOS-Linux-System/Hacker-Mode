use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::process::Command;

#[derive(Default)]
pub struct EpicProvider;

const EPIC_LOGIN_URL: &str = "https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9%26responseType%3Dcode";

/// Odpowiada (najbliżej jak się dało odtworzyć bez dostępu do zainstalowanej
/// wersji `legendary`) polom zwracanym przez `legendary list-installed --json`.
/// Pola nieznane/nieużywane są ignorowane dzięki `#[serde(default)]` na
/// poziomie `Option`.
#[derive(Debug, Deserialize)]
struct LegendaryInstalledGame {
    app_name: String,
    title: Option<String>,
    install_path: Option<String>,
    version: Option<String>,
    executable: Option<String>,
    #[serde(default)]
    is_dlc: bool,
}

impl StoreProvider for EpicProvider {
    fn platform(&self) -> Platform {
        Platform::Epic
    }

    fn is_available(&self) -> bool {
        tool_available("legendary")
    }

    fn is_logged_in(&self) -> bool {
        Command::new("legendary")
            .args(["status", "--json"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Ok(LoginFlow {
            url: Some(EPIC_LOGIN_URL.to_string()),
            instructions: "Zaloguj się na swoje konto Epic Games. Po zalogowaniu strona \
                pokaże krótki kod JSON — skopiuj wartość pola „authorizationCode” i wklej ją \
                w Hacker Mode."
                .to_string(),
            needs_code: true,
        })
    }

    fn login_submit(&self, code: &str) -> Result<(), StoreError> {
        let output = Command::new("legendary")
            .args(["auth", "--code", code])
            .output()
            .map_err(|e| StoreError::ExecFailed("legendary auth".into(), e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(StoreError::ExecFailed(
                "legendary auth".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("legendary")
            .args(["list-installed", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("legendary".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("Epic Games".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<LegendaryInstalledGame> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("legendary list-installed".into(), e.to_string()))?;

        let parsed: Vec<LegendaryInstalledGame> = parsed.into_iter().filter(|g| !g.is_dlc).collect();

        // Podobnie jak w GOG (patrz `gog.rs::fetch_covers_in_parallel`):
        // `find_cover_art` czyta lokalny plik metadanych `legendary`, ale
        // samo pobranie okładki (`cover_cache::cache_image`) jest sieciowe,
        // gdy nie ma jej jeszcze w cache Hacker Mode — więc przy sporej,
        // świeżo dodanej bibliotece robimy to równolegle zamiast po kolei.
        let app_names: Vec<String> = parsed.iter().map(|e| e.app_name.clone()).collect();
        let covers = fetch_covers_in_parallel(&app_names);

        let games = parsed
            .into_iter()
            .zip(covers)
            .map(|(entry, cover_path)| Game {
                title: entry.title.clone().unwrap_or_else(|| entry.app_name.clone()),
                id: entry.app_name,
                platform: Platform::Epic,
                installed: true,
                install_dir: entry.install_path,
                cover_path,
                playtime_minutes: None,
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["install", game_id, "-y"]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["uninstall", game_id, "-y"]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["launch", game_id]);
        Ok(cmd)
    }
}

/// Odpowiada polom, które nas interesują z `legendary list --json` (pełna
/// biblioteka POSIADANYCH gier na koncie Epic — w przeciwieństwie do
/// `legendary list-installed --json`, który widzi WYŁĄCZNIE gry już
/// pobrane na tę maszynę). Legendary utrzymuje tę komendę od dawna jako
/// odpowiednik przeglądania własnej biblioteki w kliencie Epic Games —
/// wymaga tylko bycia zalogowanym (ten sam `legendary auth`, którego już
/// używa `login_submit`), bez żadnego dodatkowego, nieudokumentowanego
/// wywołania API.
#[derive(Debug, Deserialize)]
struct LegendaryCatalogGame {
    app_name: String,
    app_title: Option<String>,
    #[serde(default)]
    is_dlc: bool,
}

/// Pełna biblioteka POSIADANYCH (nie tylko zainstalowanych) gier Epic —
/// używane przez `stores::list_all_games`, żeby dołożyć do biblioteki
/// Hacker Mode gry, których użytkownik jeszcze nie zainstalował (patrz
/// analogiczny mechanizm dla Steam: `steam::fetch_owned_games`). Ciche
/// błędy (`Vec::new()`) — brak zalogowania/`legendary` niedostępne nie
/// powinno nigdy zablokować reszty biblioteki, tylko pominąć tę jedną
/// dodatkową funkcję.
pub fn fetch_owned_games(_installed_ids: &std::collections::HashSet<String>) -> Vec<super::OwnedGame> {
    let output = Command::new("legendary").args(["list", "--json"]).output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        tracing::debug!(
            stderr = %String::from_utf8_lossy(&output.stderr),
            "Epic: `legendary list --json` nie powiodło się (prawdopodobnie brak logowania)"
        );
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<LegendaryCatalogGame> = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(err) => {
            tracing::debug!(%err, "Epic: nie udało się sparsować `legendary list --json`");
            return Vec::new();
        }
    };

    parsed
        .into_iter()
        .filter(|g| !g.is_dlc)
        .map(|g| {
            // BUGFIX: referencja do `g.id` (pole, którego `LegendaryCatalogGame`
            // w ogóle nie ma — jedyny identyfikator to `app_name`) —
            // zgłoszone przez `cargo check` (E0609). Okładka szukana PRZED
            // przeniesieniem `g.app_name` do `title` niżej, żeby uniknąć
            // też błędu "use of moved value".
            let cover_path = crate::cover_cache::build_client()
                .and_then(|client| find_cover_art(&g.app_name, &client));
            super::OwnedGame {
                id: g.app_name.clone(),
                title: g.app_title.unwrap_or(g.app_name),
                // Ta sama okładka co dla zainstalowanych gier (lokalny cache
                // metadanych `legendary`, patrz `find_cover_art`) — legendary
                // zapisuje ten plik dla KAŻDEJ gry w bibliotece, którą kiedyś
                // wylistował (`legendary list`), nie tylko dla zainstalowanych.
                cover_path,
            }
        })
        .collect()
}

/// Dociąga opis i zrzuty ekranu dla widoku szczegółów gry — BEZ żadnego
/// dodatkowego zapytania sieciowego, bo `legendary` już zapisał pełne
/// metadane katalogowe Epic (włącznie z `description` i `keyImages`) w
/// TYM SAMYM pliku cache, którego `find_cover_art` (wyżej) używa do
/// okładek — patrz jego moduł-dokumentacja. To dokładnie ten sam surowy
/// JSON, jaki Epic Games Store zwraca ze swojego katalogu, tylko już
/// pobrany i zapisany lokalnie przez `legendary`, więc nie musimy sami
/// zgadywać/odtwarzać żadnego endpointu Epic.
pub fn fetch_store_details(app_name: &str) -> Option<super::GameDetails> {
    let metadata_path = dirs::config_dir()?
        .join("legendary/metadata")
        .join(format!("{app_name}.json"));
    let content = std::fs::read_to_string(metadata_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // `keyImages` to tablica obiektów `{ "type": "...", "url": "..." }` —
    // Epic oznacza zrzuty ekranu typami `"Screenshot"` (starszy katalog)
    // albo `"OfferImageWide"`/`"OfferImageTall"` (nowszy, patrz przykłady
    // publicznych dumpów katalogu Epic używanych przez społecznościowe
    // narzędzia typu `epicgames-freegames-node`) — bierzemy WSZYSTKO
    // zaczynające się od "Screenshot", a w ich braku nic (lepsze puste niż
    // pokazanie np. loga sklepu jako "zrzutu ekranu").
    let screenshots: Vec<String> = parsed
        .get("keyImages")
        .and_then(|v| v.as_array())
        .map(|images| {
            images
                .iter()
                .filter(|img| {
                    img.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t.starts_with("Screenshot"))
                        .unwrap_or(false)
                })
                .filter_map(|img| img.get("url").and_then(|u| u.as_str()))
                .map(String::from)
                .take(6)
                .collect()
        })
        .unwrap_or_default();

    if description.is_empty() && screenshots.is_empty() {
        return None;
    }
    Some(super::GameDetails { description, screenshots })
}

/// `legendary` cache'uje pełne metadane katalogowe (włącznie z URL-ami
/// artworku) pod `~/.config/legendary/metadata/<app_name>.json` po każdym
/// `legendary list`/`legendary info` — czytamy ten plik, jeśli istnieje,
/// zamiast odpytywać API Epica bezpośrednio (które wymaga uwierzytelnienia
/// niepublicznym tokenem). Jeśli plik nie istnieje (użytkownik nigdy nie
/// odświeżył katalogu), po prostu brak okładki — nie jest to błąd krytyczny.
fn find_cover_art(app_name: &str, client: &reqwest::blocking::Client) -> Option<String> {
    let metadata_path = dirs::config_dir()?
        .join("legendary/metadata")
        .join(format!("{app_name}.json"));
    let content = std::fs::read_to_string(metadata_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let url = crate::cover_cache::find_image_url(&parsed)?;
    crate::cover_cache::cache_image(&url, &format!("epic-{app_name}"), client)
}

/// Pobiera okładki dla wielu gier Epic naraz na ograniczonej puli wątków —
/// patrz `gog.rs::fetch_covers_in_parallel` po pełny opis strategii
/// (identyczna tu, tylko nad `app_name` zamiast ID produktu GOG). Wynik
/// zachowuje kolejność wejściowego `app_names` dzięki rozłącznym,
/// ciągłym fragmentom (`chunks`/`chunks_mut`) przydzielanym każdemu
/// wątkowi, więc nie jest potrzebny `Mutex` ani sortowanie po fakcie.
fn fetch_covers_in_parallel(app_names: &[String]) -> Vec<Option<String>> {
    const MAX_PARALLEL_COVER_FETCHES: usize = 8;

    let mut covers: Vec<Option<String>> = vec![None; app_names.len()];
    if app_names.is_empty() {
        return covers;
    }

    let Some(client) = crate::cover_cache::build_client() else {
        return covers;
    };

    let worker_count = MAX_PARALLEL_COVER_FETCHES.min(app_names.len());
    let chunk_size = (app_names.len() + worker_count - 1) / worker_count;

    std::thread::scope(|scope| {
        let name_chunks = app_names.chunks(chunk_size);
        let cover_chunks = covers.chunks_mut(chunk_size);
        for (names_chunk, covers_chunk) in name_chunks.zip(cover_chunks) {
            let client = &client;
            scope.spawn(move || {
                for (name, slot) in names_chunk.iter().zip(covers_chunk.iter_mut()) {
                    *slot = find_cover_art(name, client);
                }
            });
        }
    });

    covers
}
