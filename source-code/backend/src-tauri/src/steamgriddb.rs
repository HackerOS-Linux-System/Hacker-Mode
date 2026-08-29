use serde::Deserialize;

const API_BASE: &str = "https://www.steamgriddb.com/api/v2";

#[derive(Debug, Deserialize)]
struct AutocompleteResponse {
    success: bool,
    data: Vec<AutocompleteEntry>,
}

#[derive(Debug, Deserialize)]
struct AutocompleteEntry {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct GridsResponse {
    success: bool,
    data: Vec<GridEntry>,
}

#[derive(Debug, Deserialize)]
struct GridEntry {
    url: String,
}

/// Szuka okładki (grid 600x900, ten sam format proporcji co Steam/GOG w
/// reszcie Hacker Mode) dla gry o danym tytule i, jeśli znajdzie, pobiera
/// ją do lokalnego cache Hacker Mode przez `cover_cache::cache_image`
/// (dokładnie tak jak robią to Epic/GOG/Amazon dla swoich okładek — więc
/// wynikowa ścieżka działa identycznie w `resolveCoverSrc` po stronie
/// frontendu).
///
/// `cache_key` powinien być unikalny w obrębie Hacker Mode (patrz
/// wywołania w `ea.rs`/`battlenet.rs`: `"ea-<nazwa_katalogu>"`/
/// `"battlenet-<nazwa_katalogu>"`) — to samo `cache_key`, co dla
/// wszystkich innych źródeł okładek w `cover_cache.rs`.
pub fn fetch_cover_by_title(
    title: &str,
    cache_key: &str,
    api_key: &str,
    client: &reqwest::blocking::Client,
) -> Option<String> {
    if api_key.trim().is_empty() || title.trim().is_empty() {
        return None;
    }

    let game_id = search_first_match(title, api_key, client)?;
    let grid_url = fetch_first_grid_url(game_id, api_key, client)?;
    crate::cover_cache::cache_image(&grid_url, cache_key, client)
}

fn auth_header_value(api_key: &str) -> String {
    format!("Bearer {api_key}")
}

fn search_first_match(title: &str, api_key: &str, client: &reqwest::blocking::Client) -> Option<u64> {
    let url = format!(
        "{API_BASE}/search/autocomplete/{}",
        crate::commands::stores::steam::urlencoding_minimal(title)
    );
    let response = client
        .get(&url)
        .header("Authorization", auth_header_value(api_key))
        .send()
        .ok()?;
    if !response.status().is_success() {
        tracing::debug!(status = %response.status(), title, "SteamGridDB: wyszukiwanie nie powiodło się");
        return None;
    }
    let body = response.text().ok()?;
    parse_autocomplete_response(&body)
}

/// Rdzeń `search_first_match` — parsowanie wydzielone z samego zapytania
/// HTTP, żeby dało się je przetestować na przykładowym JSON-ie (patrz
/// `mod tests`) bez uruchamiania prawdziwego serwera HTTP. Zwraca `None`
/// zarówno gdy JSON jest nieprawidłowy, jak i gdy API zwróciło
/// `"success": false` albo pustą listę wyników — z zewnątrz
/// (`fetch_cover_by_title`) te trzy przypadki i tak oznaczają dokładnie to
/// samo: "nic nie znaleziono, jedziemy dalej bez okładki".
fn parse_autocomplete_response(body: &str) -> Option<u64> {
    let parsed: AutocompleteResponse = serde_json::from_str(body).ok()?;
    if !parsed.success {
        return None;
    }
    parsed.data.first().map(|entry| entry.id)
}

fn fetch_first_grid_url(game_id: u64, api_key: &str, client: &reqwest::blocking::Client) -> Option<String> {
    // `dimensions=600x900` ogranicza wynik do proporcji okładki biblioteki
    // (ten sam format, którego SteamGridDB używa dla własnych "Grids" w
    // pionowym układzie) — bez tego API zwraca też szerokie
    // banery/kafelki, które źle wyglądałyby w siatce biblioteki Hacker
    // Mode obok okładek Steam/GOG/Epic.
    let url = format!("{API_BASE}/grids/game/{game_id}?dimensions=600x900");
    let response = client
        .get(&url)
        .header("Authorization", auth_header_value(api_key))
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    parse_grids_response(&body)
}

/// Rdzeń `fetch_first_grid_url` — patrz `parse_autocomplete_response` po
/// wyjaśnienie, dlaczego parsowanie jest osobną, testowalną funkcją.
fn parse_grids_response(body: &str) -> Option<String> {
    let parsed: GridsResponse = serde_json::from_str(body).ok()?;
    if !parsed.success {
        return None;
    }
    parsed.data.first().map(|entry| entry.url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kształt prawdziwej odpowiedzi `GET /search/autocomplete/<term>` z
    /// oficjalnej dokumentacji SteamGridDB (https://www.steamgriddb.com/api/v2#tag/search),
    /// skrócony do pól, których faktycznie używamy — reszta pól
    /// (`types`, `verified`, ...) jest w prawdziwej odpowiedzi, ale
    /// `#[derive(Deserialize)]` domyślnie ignoruje nieznane pola, więc nie
    /// musimy ich tu odtwarzać, żeby test był miarodajny.
    const SAMPLE_AUTOCOMPLETE_HIT: &str = r#"{
        "success": true,
        "data": [
            { "id": 5563, "name": "Half-Life 2", "types": ["steam"], "verified": true },
            { "id": 9999, "name": "Half-Life 2: Episode One", "types": ["steam"], "verified": true }
        ]
    }"#;

    const SAMPLE_AUTOCOMPLETE_EMPTY: &str = r#"{ "success": true, "data": [] }"#;

    const SAMPLE_AUTOCOMPLETE_ERROR: &str = r#"{ "success": false, "errors": ["Invalid API key"] }"#;

    const SAMPLE_GRIDS_HIT: &str = r#"{
        "success": true,
        "data": [
            {
                "id": 121, "score": 1, "style": "alternate", "width": 600, "height": 900,
                "url": "https://cdn2.steamgriddb.com/grid/aaaabbbbccccdddd.png",
                "thumb": "https://cdn2.steamgriddb.com/thumb/aaaabbbbccccdddd.png",
                "tags": [], "author": { "name": "ktos", "steam64": "1", "avatar": "" }
            }
        ]
    }"#;

    const SAMPLE_GRIDS_EMPTY: &str = r#"{ "success": true, "data": [] }"#;

    #[test]
    fn parses_first_autocomplete_match() {
        assert_eq!(parse_autocomplete_response(SAMPLE_AUTOCOMPLETE_HIT), Some(5563));
    }

    #[test]
    fn autocomplete_with_no_results_returns_none() {
        assert_eq!(parse_autocomplete_response(SAMPLE_AUTOCOMPLETE_EMPTY), None);
    }

    #[test]
    fn autocomplete_success_false_returns_none_even_with_data_shape_present() {
        assert_eq!(parse_autocomplete_response(SAMPLE_AUTOCOMPLETE_ERROR), None);
    }

    #[test]
    fn autocomplete_garbage_input_returns_none_instead_of_panicking() {
        assert_eq!(parse_autocomplete_response("to nie jest JSON"), None);
        assert_eq!(parse_autocomplete_response(""), None);
    }

    #[test]
    fn parses_first_grid_url() {
        assert_eq!(
            parse_grids_response(SAMPLE_GRIDS_HIT),
            Some("https://cdn2.steamgriddb.com/grid/aaaabbbbccccdddd.png".to_string())
        );
    }

    #[test]
    fn grids_with_no_results_returns_none() {
        assert_eq!(parse_grids_response(SAMPLE_GRIDS_EMPTY), None);
    }

    #[test]
    fn fetch_cover_by_title_short_circuits_on_empty_api_key_without_network() {
        // Klient reqwest bez skonfigurowanego proxy/hosta i tak by nie
        // zdążył nic wysłać w czasie testu, ale i tak sprawdzamy, że dla
        // pustego klucza funkcja wraca `None` NATYCHMIAST — patrz
        // wczesny `return None` w `fetch_cover_by_title` przed
        // zbudowaniem jakiegokolwiek żądania.
        let client = reqwest::blocking::Client::new();
        assert_eq!(fetch_cover_by_title("Diablo IV", "battlenet-diablo-iv", "", &client), None);
        assert_eq!(fetch_cover_by_title("", "battlenet-empty-title", "some-key", &client), None);
    }
}
