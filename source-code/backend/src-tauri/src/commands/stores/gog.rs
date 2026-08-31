use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
pub struct GogProvider;

const GOG_LOGIN_URL: &str = "https://auth.gog.com/auth?client_id=46899977096215655&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code&layout=client2";

#[derive(Debug, Deserialize)]
struct GogdlInstalledGame {
    id: String,
    title: Option<String>,
    install_path: Option<String>,
    #[serde(default)]
    dlcs: Vec<serde_json::Value>,
}

/// Kształt pliku, w którym `gogdl auth` zapisuje token dostępu — patrz
/// `login_submit` niżej (`gogdl ... auth --code <code>` pisze go pod
/// `Self::config_dir()/auth.json`). Pole `access_token` potwierdzone
/// niezależnie w logach Heroic Games Launcher (który używa dokładnie
/// tego samego `gogdl`/`heroic-gogdl` co ten provider) — np. zgłoszenia
/// błędów na GitHubie pokazujące surową zawartość tego pliku:
/// `{"access_token":"...","expires_in":3600,"token_type":"bearer",...}`.
/// Reszta pól (`refresh_token`, `expires_in`, ...) nas tu nie interesuje —
/// `gogdl` sam odświeża token przy każdym wywołaniu `auth`, Hacker Mode
/// tylko go CZYTA do własnych zapytań HTTP, nigdy nie odświeża.
#[derive(Debug, Deserialize)]
struct GogAuthFile {
    access_token: String,
}

impl GogProvider {
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .join("hacker-mode/gog")
    }
}

impl StoreProvider for GogProvider {
    fn platform(&self) -> Platform {
        Platform::Gog
    }

    fn is_available(&self) -> bool {
        tool_available("gogdl")
    }

    fn is_logged_in(&self) -> bool {
        Self::config_dir().join("auth.json").exists()
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Ok(LoginFlow {
            url: Some(GOG_LOGIN_URL.to_string()),
            instructions: "Zaloguj się na swoje konto GOG.com. Po zalogowaniu strona \
                przekieruje na „embed.gog.com” z parametrem „code” w adresie — Hacker Mode \
                wykryje go automatycznie."
                .to_string(),
            needs_code: true,
        })
    }

    fn login_submit(&self, code: &str) -> Result<(), StoreError> {
        let output = Command::new("gogdl")
            .args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["auth", "--code", code])
            .output()
            .map_err(|e| StoreError::ExecFailed("gogdl auth".into(), e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(StoreError::ExecFailed(
                "gogdl auth".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("gogdl")
            .args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["list-installed"])
            .output()
            .map_err(|e| StoreError::ExecFailed("gogdl".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("GOG".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<GogdlInstalledGame> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("gogdl list-installed".into(), e.to_string()))?;

        // Okładki pobieramy z `api.gog.com` (sieciowo) dla każdej gry z
        // osobna — przy sporej bibliotece i pustym lokalnym cache
        // (`cover_cache`) robienie tego po kolei potrafi zauważalnie
        // spowolnić `list_games`. Rozdzielamy więc pobieranie na kilka
        // wątków naraz (patrz `fetch_covers_in_parallel`), zamiast
        // odpytywać API okładka po okładce.
        let ids: Vec<String> = parsed.iter().map(|e| e.id.clone()).collect();
        let covers = fetch_covers_in_parallel(&ids);

        let games = parsed
            .into_iter()
            .zip(covers)
            .map(|(entry, cover_path)| Game {
                title: entry.title.clone().unwrap_or_else(|| format!("Gra GOG #{}", entry.id)),
                cover_path,
                id: entry.id,
                platform: Platform::Gog,
                installed: true,
                install_dir: entry.install_path,
                playtime_minutes: None,
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["download", game_id]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["uninstall", game_id]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["launch", game_id]);
        Ok(cmd)
    }
}

/// GOG wystawia publiczny (nieuwierzytelniony), od dawna stabilny endpoint
/// `api.gog.com/products/<id>` używany przez wiele narzędzi społecznościowych
/// (GOGDB i inne) do pobierania metadanych katalogowych, włącznie z
/// artworkiem. Nie wymaga logowania, więc można go odpytać zawsze — w
/// przeciwieństwie do Epic, gdzie okładki są dostępne tylko z lokalnego
/// cache `legendary` po zalogowaniu.
///
/// Przyjmuje współdzielony `client`, żeby dało się go wywoływać równolegle
/// z jednym poolem połączeń — patrz `fetch_covers_in_parallel`.
fn find_cover_art(product_id: &str, client: &reqwest::blocking::Client) -> Option<String> {
    let url = format!("https://api.gog.com/products/{product_id}?expand=images");
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    let image_url = crate::cover_cache::find_image_url(&parsed)?;
    // Adresy z GOG API bywają bez schematu (`//images.gog.com/...`).
    let image_url = if image_url.starts_with("//") {
        format!("https:{image_url}")
    } else {
        image_url
    };
    crate::cover_cache::cache_image(&image_url, &format!("gog-{product_id}"), client)
}

/// Pobiera okładki dla wielu ID gier GOG naraz, rozdzielając pracę na
/// ograniczoną pulę wątków zamiast pobierać okładka-po-okładce.
///
/// Zwraca `Vec<Option<String>>` w **dokładnie tej samej kolejności** co
/// wejściowe `product_ids` (każdy wątek dostaje rozłączny, ciągły
/// fragment — `chunks_mut`/`chunks` — więc wynik nie wymaga sortowania ani
/// dodatkowej synchronizacji przez `Mutex`).
///
/// Liczba wątków jest ograniczona do `MAX_PARALLEL_COVER_FETCHES`, żeby przy
/// dużej bibliotece nie odpalić naraz setek połączeń do `api.gog.com` (co
/// mogłoby też trafić na throttling po stronie API).
fn fetch_covers_in_parallel(product_ids: &[String]) -> Vec<Option<String>> {
    const MAX_PARALLEL_COVER_FETCHES: usize = 8;

    let mut covers: Vec<Option<String>> = vec![None; product_ids.len()];
    if product_ids.is_empty() {
        return covers;
    }

    let Some(client) = crate::cover_cache::build_client() else {
        // Nie udało się zbudować klienta HTTP (skrajnie rzadki przypadek) —
        // po prostu brak okładek, tak jak przy pojedynczym błędzie sieci.
        return covers;
    };

    let worker_count = MAX_PARALLEL_COVER_FETCHES.min(product_ids.len());
    // Ręczne zaokrąglenie w górę zamiast `usize::div_ceil` (stabilne dopiero
    // od Rust 1.73) — ten kod nie przeszedł jeszcze przez kompilator na
    // żadnej konkretnej wersji toolchaina (patrz README), więc unikamy
    // zależności od nowszych stabilizacji, gdzie nie jest to konieczne.
    let chunk_size = (product_ids.len() + worker_count - 1) / worker_count;

    std::thread::scope(|scope| {
        let id_chunks = product_ids.chunks(chunk_size);
        let cover_chunks = covers.chunks_mut(chunk_size);
        for (ids_chunk, covers_chunk) in id_chunks.zip(cover_chunks) {
            let client = &client;
            scope.spawn(move || {
                for (id, slot) in ids_chunk.iter().zip(covers_chunk.iter_mut()) {
                    *slot = find_cover_art(id, client);
                }
            });
        }
    });

    covers
}

/// Pełna biblioteka POSIADANYCH gier GOG — w przeciwieństwie do
/// Epic (`legendary list`) i Amazon (`nile library list`), `gogdl` NIE ma
/// żadnej komendy do tego (patrz README, sekcja "Zakres wsparcia
/// platform"). Jedyny sposób: odpytać bezpośrednio prywatne API GOG
/// (`embed.gog.com/user/data/games`), tym samym tokenem dostępu, który
/// `gogdl` i tak już zapisał lokalnie po zalogowaniu (`Self::config_dir()
/// /auth.json`, patrz `GogAuthFile`) — dokładnie ten sam wzorzec co
/// oficjalny klient GOG Galaxy i narzędzia społecznościowe (Heroic,
/// `goggle`, patrz komentarz przy `GogAuthFile`).
///
/// Endpoint zwraca WYŁĄCZNIE listę numerycznych ID
/// (`{"owned": [123, 456, ...]}`, potwierdzone niezależnie w
/// nieoficjalnej dokumentacji community `gogapidocs` i w kodzie źródłowym
/// kilku otwartoźródłowych klientów GOG) — bez tytułów ani okładek, więc
/// dopiero PO odjęciu tego, co już mamy zainstalowane (`installed_ids`,
/// przekazane przez `stores::merge_owned_but_not_installed`, żeby nie
/// odpytywać po tytuł/okładkę gier, które i tak już są na liście),
/// dociągamy resztę szczegółów przez ten sam publiczny
/// `api.gog.com/products/<id>`, którego już używa `find_cover_art`
/// wyżej — równolegle, tą samą pulą wątków.
pub fn fetch_owned_games(installed_ids: &std::collections::HashSet<String>) -> Vec<super::OwnedGame> {
    let Some(access_token) = read_access_token() else {
        return Vec::new();
    };
    let Some(client) = crate::cover_cache::build_client() else {
        return Vec::new();
    };

    let response = client
        .get("https://embed.gog.com/user/data/games")
        .header("Authorization", format!("Bearer {access_token}"))
        .send();
    let Ok(response) = response else {
        return Vec::new();
    };
    if !response.status().is_success() {
        tracing::debug!(status = %response.status(), "GOG: nie udało się pobrać listy posiadanych gier (embed.gog.com)");
        return Vec::new();
    }
    let Ok(parsed) = response.json::<serde_json::Value>() else {
        return Vec::new();
    };
    let owned_ids = parse_owned_ids(&parsed);

    let missing_ids: Vec<String> = owned_ids.into_iter().filter(|id| !installed_ids.contains(id)).collect();
    if missing_ids.is_empty() {
        return Vec::new();
    }

    fetch_owned_game_details_in_parallel(&missing_ids, &client)
}

/// Rdzeń parsowania odpowiedzi `embed.gog.com/user/data/games`
/// (`{"owned": [123, 456, ...]}`) — wydzielone z `fetch_owned_games`, żeby
/// dało się je przetestować na przykładowym JSON-ie bez wykonywania
/// prawdziwego zapytania HTTP (patrz `mod tests`).
fn parse_owned_ids(parsed: &serde_json::Value) -> Vec<String> {
    parsed
        .get("owned")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).map(|id| id.to_string()).collect())
        .unwrap_or_default()
}

/// Czyta `access_token` z `auth.json` zapisanego przez `gogdl auth` —
/// patrz `GogAuthFile`. Zwraca `None` (zamiast błędu), jeśli plik nie
/// istnieje/jest uszkodzony — to zwyczajnie oznacza "nie zalogowano",
/// dokładnie tak jak `is_logged_in` niżej sprawdza istnienie tego samego
/// pliku.
fn read_access_token() -> Option<String> {
    let path = GogProvider::config_dir().join("auth.json");
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: GogAuthFile = serde_json::from_str(&content).ok()?;
    Some(parsed.access_token)
}

/// Dociąga tytuł i okładkę dla gier posiadanych, ale niezainstalowanych —
/// jedno zapytanie na grę do `api.gog.com/products/<id>`, na tej samej
/// ograniczonej puli wątków co `fetch_covers_in_parallel` (patrz jej
/// dokumentacja po pełny opis strategii). Różnica: tu potrzebujemy Z TEGO
/// SAMEGO zapytania zarówno tytułu, jak i okładki (zamiast tylko okładki
/// jak w `find_cover_art`), więc to osobna, nieco inna funkcja zamiast
/// ponownego użycia `find_cover_art`.
fn fetch_owned_game_details_in_parallel(ids: &[String], client: &reqwest::blocking::Client) -> Vec<super::OwnedGame> {
    const MAX_PARALLEL_FETCHES: usize = 8;

    let mut results: Vec<Option<super::OwnedGame>> = vec![None; ids.len()];
    let worker_count = MAX_PARALLEL_FETCHES.min(ids.len());
    let chunk_size = (ids.len() + worker_count - 1) / worker_count;

    std::thread::scope(|scope| {
        let id_chunks = ids.chunks(chunk_size);
        let result_chunks = results.chunks_mut(chunk_size);
        for (ids_chunk, results_chunk) in id_chunks.zip(result_chunks) {
            scope.spawn(move || {
                for (id, slot) in ids_chunk.iter().zip(results_chunk.iter_mut()) {
                    *slot = fetch_owned_game_details(id, client);
                }
            });
        }
    });

    results.into_iter().flatten().collect()
}

/// Dociąga tytuł+okładkę dla JEDNEGO produktu GOG — patrz
/// `fetch_owned_game_details_in_parallel` po kontekst wywołania.
/// Cache'owane na dobę (`net_cache`, ten sam TTL co ProtonDB — tytuł i
/// okładka gry praktycznie nigdy się nie zmieniają) pod kluczem
/// `product_id`, NIEZALEŻNIE od tego, czy `installed_ids` się zmieni
/// między kolejnymi odświeżeniami biblioteki — raz pobrane szczegóły
/// danej gry pozostają w cache'u i są reużywane, nawet jeśli przy
/// następnym odświeżeniu ta sama gra znowu wypadnie z listy
/// "brakujących" (bo cache jest per-gra, nie per-zestaw-brakujących-gier).
fn fetch_owned_game_details(product_id: &str, client: &reqwest::blocking::Client) -> Option<super::OwnedGame> {
    const TTL_SECONDS: u64 = 60 * 60 * 24;
    crate::net_cache::get_or_fetch("gog_owned_game_details.json", product_id, TTL_SECONDS, || {
        fetch_owned_game_details_uncached(product_id, client)
    })
}

fn fetch_owned_game_details_uncached(product_id: &str, client: &reqwest::blocking::Client) -> Option<super::OwnedGame> {
    let url = format!("https://api.gog.com/products/{product_id}?expand=images");
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    let title = parsed.get("title").and_then(|v| v.as_str())?.to_string();
    let cover_path = crate::cover_cache::find_image_url(&parsed).and_then(|url| {
        let url = if url.starts_with("//") { format!("https:{url}") } else { url };
        crate::cover_cache::cache_image(&url, &format!("gog-{product_id}"), client)
    });

    Some(super::OwnedGame { id: product_id.to_string(), title, cover_path })
}

/// Rozszerza ten sam publiczny endpoint co `find_cover_art`
/// (`api.gog.com/products/<id>`) o `description`/`screenshots` —
/// oficjalne, udokumentowane parametry `expand` tego API (patrz np.
/// referencja pól na GOGDB, community-owym katalogu API GOG). Osobne
/// zapytanie od okładki (nie łączymy z `expand=images,description,
/// screenshots` w jednym), bo to wywoływane rzadko — tylko na żądanie, przy
/// otwarciu widoku szczegółów jednej gry (`commands::fetch_game_details`)
/// — w przeciwieństwie do okładek, które trzeba pobrać dla całej biblioteki
/// od razu przy `list_games`.
pub fn fetch_store_details(product_id: &str) -> Option<super::GameDetails> {
    let client = crate::cover_cache::build_client()?;
    let url = format!("https://api.gog.com/products/{product_id}?expand=description,screenshots");
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    parse_store_details(&parsed)
}

/// Rdzeń `fetch_store_details` — wydzielone parsowanie, testowalne na
/// przykładowym JSON-ie bez wykonywania zapytania HTTP (patrz `mod
/// tests`).
fn parse_store_details(parsed: &serde_json::Value) -> Option<super::GameDetails> {
    // GOG zwraca opis jako HTML (`description.full`/`description.lead`) —
    // widok szczegółów w Hacker Mode (`GameDetail.tsx`) renderuje opis jako
    // zwykły tekst, więc obcinamy tagi bardzo prostym filtrem zamiast
    // ciągnąć zależność do pełnego parsera HTML dla jednego pola.
    let raw_description = parsed
        .get("description")
        .and_then(|d| d.get("full").or_else(|| d.get("lead")))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let description = strip_html_tags(raw_description);

    // `screenshots` to tablica adresów bazowych bez rozszerzenia/rozmiaru
    // (np. `//images.gog-statics.com/xxx`) — GOG doklejane rozmiary przez
    // szablon w URL-u (`_{formatter}`), ale sam bazowy adres (z dodanym
    // schematem) już działa jako pełnoprawny obraz, więc nie odtwarzamy tu
    // całego, nieudokumentowanego systemu szablonów rozmiarów.
    let screenshots: Vec<String> = parsed
        .get("screenshots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str())
                .map(|s| if s.starts_with("//") { format!("https:{s}") } else { s.to_string() })
                .take(6)
                .collect()
        })
        .unwrap_or_default();

    if description.is_empty() && screenshots.is_empty() {
        return None;
    }
    Some(super::GameDetails { description, screenshots })
}

/// Bardzo prosty filtr `<tag>` → nic, wystarczający dla opisów GOG (które
/// używają tylko podstawowego formatowania: `<p>`, `<br>`, `<b>`, listy) —
/// celowo NIE jest to pełny parser HTML (żadna nowa zależność), więc
/// nietypowe znaczniki (np. `<script>`, którego GOG i tak nie zwraca w tym
/// polu) zostałyby po prostu usunięte razem z zawartością tagu, nigdy
/// wykonane.
///
/// BUGFIX (zgłoszone przez `cargo test` w CI — `logs_Hacker-Mode.zip`):
/// wcześniejsza wersja po prostu POMIJAŁA znaki wewnątrz `<...>`, bez
/// wstawienia w ich miejsce spacji — dla tagów typu `<br>`/`<p>`, które w
/// HTML same w sobie PEŁNIĄ rolę separatora (nawet bez otaczających
/// spacji w źródle, jak `"Linia<br>druga"`), dawało to sklejony tekst
/// (`"Liniadruga"`) zamiast oddzielonego (`"Linia druga"`). Naprawione
/// przez wstawienie spacji przy KAŻDYM zamknięciu tagu — ewentualne
/// nadmiarowe/podwójne spacje (np. gdy w źródle i tak była spacja tuż
/// obok tagu) są i tak sprzątane przez `split_whitespace().join(" ")`
/// niżej, więc to bezpieczne uproszczenie: lepiej wstawić spację zawsze i
/// pozwolić normalizacji ją ewentualnie scalić, niż zgadywać, które tagi
/// "na pewno" nie potrzebują separatora.
fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Wynik wyszukiwania w katalogu GOG — używane przez frontend
/// (`Store.tsx`) do pokazania gier z całego sklepu GOG, nie tylko tych już
/// posiadanych, analogicznie do istniejącego wyszukiwania Steam.
#[derive(Debug, serde::Serialize)]
pub struct StoreSearchResult {
    pub product_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    /// Krótki, "ładny" identyfikator używany w adresach na gog.com
    /// (`https://www.gog.com/game/<slug>`) — obecny w odpowiedzi
    /// `catalog.gog.com`, gdy sklep w ogóle ma stronę produktu (DLC/pakiety
    /// czasem go nie mają). Używane przez `open_store_page` w
    /// `commands/mod.rs`, żeby otworzyć stronę produktu w przeglądarce —
    /// GOG (w przeciwieństwie do Steam) nie ma na Linuksie działającego
    /// klienta z własnym schematem `gog://`, więc jedyna opcja to zwykła
    /// strona WWW.
    pub slug: Option<String>,
}

/// Przeszukuje publiczny katalog GOG przez `catalog.gog.com` — ten sam,
/// nieuwierzytelniony endpoint, którego używa sama strona gog.com do
/// wyszukiwania/przeglądania sklepu (udokumentowany przez community, m.in.
/// referencję pól na GOGDB — parametr `query=like:<fraza>` filtruje po
/// tytule). W przeciwieństwie do `api.gog.com/products/<id>` (pojedyncza,
/// znana już gra) to jest właściwy odpowiednik `steam::search_store` —
/// przeszukiwanie całego katalogu sklepu, nie tylko biblioteki
/// użytkownika. Nie wymaga logowania.
pub fn search_store(query: &str) -> Vec<StoreSearchResult> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Some(client) = crate::cover_cache::build_client() else {
        return Vec::new();
    };

    let url = format!(
        "https://catalog.gog.com/v1/catalog?limit=20&query=like:{}&order=desc:trending&productType=in:game",
        super::steam::urlencoding_minimal(trimmed)
    );
    let Ok(response) = client.get(&url).send() else {
        return Vec::new();
    };
    if !response.status().is_success() {
        return Vec::new();
    }
    let Ok(parsed) = response.json::<serde_json::Value>() else {
        return Vec::new();
    };
    parse_search_results(&parsed)
}

/// Rdzeń `search_store` — patrz `parse_owned_ids`/`parse_store_details`
/// po ten sam powód wydzielenia (testowalność bez sieci). Zaakceptowana
/// odpowiedź może brakować pola `id`/`title` w pojedynczym produkcie
/// (`filter_map`, `?` w środku) — pomijamy TYLKO ten jeden wpis zamiast
/// odrzucać całą resztę wyników przez jeden nietypowy rekord.
fn parse_search_results(parsed: &serde_json::Value) -> Vec<StoreSearchResult> {
    parsed
        .get("products")
        .and_then(|v| v.as_array())
        .map(|products| {
            products
                .iter()
                .filter_map(|p| {
                    let product_id = p.get("id").and_then(|v| v.as_str())?.to_string();
                    let title = p.get("title").and_then(|v| v.as_str())?.to_string();
                    let cover_url = p
                        .get("coverVertical")
                        .or_else(|| p.get("coverHorizontal"))
                        .and_then(|v| v.as_str())
                        .map(|s| if s.starts_with("//") { format!("https:{s}") } else { s.to_string() });
                    let slug = p.get("slug").and_then(|v| v.as_str()).map(String::from);
                    Some(StoreSearchResult { product_id, title, cover_url, slug })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_owned_ids_as_strings() {
        let json: serde_json::Value = serde_json::from_str(r#"{"owned": [1207658691, 1207658713]}"#).unwrap();
        assert_eq!(parse_owned_ids(&json), vec!["1207658691", "1207658713"]);
    }

    #[test]
    fn parses_owned_ids_empty_list() {
        let json: serde_json::Value = serde_json::from_str(r#"{"owned": []}"#).unwrap();
        assert!(parse_owned_ids(&json).is_empty());
    }

    #[test]
    fn parses_owned_ids_missing_field_returns_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        assert!(parse_owned_ids(&json).is_empty());
    }

    #[test]
    fn strips_basic_html_tags_from_description() {
        assert_eq!(
            strip_html_tags("<p>Epicka <b>przygoda</b> w otwartym świecie.</p>"),
            "Epicka przygoda w otwartym świecie."
        );
    }

    #[test]
    fn strip_html_tags_collapses_whitespace_left_by_removed_tags() {
        assert_eq!(strip_html_tags("Linia<br>druga<br/>linia"), "Linia druga linia");
    }

    #[test]
    fn parse_store_details_extracts_description_and_screenshots() {
        // Kształt oparty na dokumentacji community `api.gog.com/products/<id>
        // ?expand=description,screenshots` — patrz komentarz przy
        // `fetch_store_details`. Niezweryfikowane na żywym wywołaniu (patrz
        // README, "Do zrobienia dalej") — ten test pilnuje TYLKO tego, że
        // parser poprawnie obsługuje kształt, jaki dokumentacja opisuje, nie
        // że ten kształt na pewno jest aktualny.
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "description": { "lead": "Krótko.", "full": "<p>Pełny <b>opis</b> gry.</p>" },
                "screenshots": [
                    "//images.gog-statics.com/aaa_{formatter}.jpg",
                    "https://images.gog-statics.com/bbb_{formatter}.jpg"
                ]
            }"#,
        )
        .unwrap();

        let details = parse_store_details(&json).unwrap();
        assert_eq!(details.description, "Pełny opis gry.");
        assert_eq!(
            details.screenshots,
            vec![
                "https://images.gog-statics.com/aaa_{formatter}.jpg".to_string(),
                "https://images.gog-statics.com/bbb_{formatter}.jpg".to_string(),
            ]
        );
    }

    #[test]
    fn parse_store_details_falls_back_to_lead_when_full_is_missing() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"description": {"lead": "Tylko krótki opis."}}"#).unwrap();
        assert_eq!(parse_store_details(&json).unwrap().description, "Tylko krótki opis.");
    }

    #[test]
    fn parse_store_details_returns_none_when_nothing_useful_present() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        assert!(parse_store_details(&json).is_none());
    }

    #[test]
    fn parse_store_details_caps_screenshots_at_six() {
        let urls: Vec<String> = (0..10).map(|i| format!("\"https://example.com/{i}.jpg\"")).collect();
        let json_str = format!(r#"{{"screenshots": [{}]}}"#, urls.join(","));
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parse_store_details(&json).unwrap().screenshots.len(), 6);
    }

    #[test]
    fn parse_search_results_extracts_all_fields() {
        // Kształt oparty na referencji pól `catalog.gog.com` z GOGDB —
        // patrz komentarz przy `search_store` i README ("Do zrobienia
        // dalej") po tę samą uwagę o braku weryfikacji na żywym wywołaniu.
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "products": [
                    {
                        "id": "1207658785",
                        "title": "The Witcher 3: Wild Hunt",
                        "slug": "the_witcher_3_wild_hunt",
                        "coverVertical": "//images.gog-statics.com/witcher3.jpg"
                    },
                    { "id": "999", "title": "Gra bez okładki" }
                ]
            }"#,
        )
        .unwrap();

        let results = parse_search_results(&json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].product_id, "1207658785");
        assert_eq!(results[0].title, "The Witcher 3: Wild Hunt");
        assert_eq!(results[0].slug.as_deref(), Some("the_witcher_3_wild_hunt"));
        assert_eq!(results[0].cover_url.as_deref(), Some("https://images.gog-statics.com/witcher3.jpg"));
        assert_eq!(results[1].cover_url, None);
        assert_eq!(results[1].slug, None);
    }

    #[test]
    fn parse_search_results_skips_entries_missing_required_fields() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"products": [{"title": "Brak ID"}, {"id": "1"}]}"#).unwrap();
        // Pierwszy wpis (brak "id") jest pomijany, drugi (brak "title")
        // też — `title` jest wymagane przez `?` w `parse_search_results`
        // dokładnie tak samo jak `id`.
        assert!(parse_search_results(&json).is_empty());
    }

    #[test]
    fn parse_search_results_missing_products_field_returns_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        assert!(parse_search_results(&json).is_empty());
    }
}
