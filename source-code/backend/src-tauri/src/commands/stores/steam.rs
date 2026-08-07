use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default)]
pub struct SteamProvider;

impl SteamProvider {
    /// Steam (natywny albo Flatpak) domyślnie trzyma dane w jednym z tych
    /// katalogów — sprawdzamy po kolei.
    fn root_candidates() -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            home.join(".steam/steam"),
            home.join(".steam/root"),
            home.join(".local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.steam/steam"),
        ]
    }

    fn find_root() -> Option<PathBuf> {
        Self::root_candidates().into_iter().find(|p| p.exists())
    }

    fn library_folders(root: &Path) -> Vec<PathBuf> {
        let mut libraries = vec![root.join("steamapps")];
        let vdf_path = root.join("steamapps/libraryfolders.vdf");
        if let Ok(content) = std::fs::read_to_string(&vdf_path) {
            let parsed = VdfValue::parse(&content);
            if let Some(map) = parsed.as_map() {
                for (_key, value) in map {
                    // Struktura: "0" => { "path" => "...", ... }, "1" => {...}
                    if let Some(entry) = value.as_map() {
                        if let Some(path_str) = entry.get("path").and_then(VdfValue::as_str) {
                            libraries.push(PathBuf::from(path_str).join("steamapps"));
                        }
                    }
                }
            }
        }
        libraries
    }
}

/// Steam trzyma lokalny cache okładek/artworku bibliotek w
/// `appcache/librarycache`. Format nazw plików zmieniał się na przestrzeni
/// lat (starszy: `<appid>_library_600x900.jpg` bezpośrednio w
/// `librarycache/`; nowszy: `librarycache/<appid>/library_600x900.jpg`),
/// więc sprawdzamy oba warianty i kilka rozszerzeń.
fn find_cover_art(root: &Path, appid: &str) -> Option<String> {
    let cache_dir = root.join("appcache").join("librarycache");
    let candidates = [
        cache_dir.join(format!("{appid}_library_600x900.jpg")),
        cache_dir.join(format!("{appid}_library_600x900.png")),
        cache_dir.join(appid).join("library_600x900.jpg"),
        cache_dir.join(appid).join("library_600x900.png"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned())
}

impl StoreProvider for SteamProvider {
    fn platform(&self) -> Platform {
        Platform::Steam
    }

    fn is_available(&self) -> bool {
        tool_available("steam") || tool_available("flatpak") || Self::find_root().is_some()
    }

    fn is_logged_in(&self) -> bool {
        // Steam zarządza logowaniem samodzielnie (własne GUI); tu jedynie
        // sprawdzamy, czy `loginusers.vdf` ma choć jeden wpis, żeby dać
        // UI sensowną wskazówkę ("otwórz Steam i zaloguj się").
        Self::find_root()
            .map(|root| {
                std::fs::read_to_string(root.join("config/loginusers.vdf"))
                    .map(|c| c.contains("\"AccountName\""))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        // Steam ma własne, natywne GUI logowania — nie ma tu żadnego kodu
        // do wklejenia. Po prostu odpalamy klienta.
        Ok(LoginFlow {
            url: None,
            instructions: "Otwórz klienta Steam i zaloguj się na swoje konto — Hacker Mode \
                automatycznie wykryje zainstalowane gry po zalogowaniu."
                .to_string(),
            needs_code: false,
        })
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        self.launch_command_with_action(game_id, "install")
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        self.launch_command_with_action(game_id, "uninstall")
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let root = Self::find_root().ok_or_else(|| {
            StoreError::ToolNotInstalled("katalog danych Steam (~/.steam/steam)".into())
        })?;

        let mut games = Vec::new();
        for lib in Self::library_folders(&root) {
            let entries = match std::fs::read_dir(&lib) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
                    .unwrap_or(false);
                if !is_manifest {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let parsed = VdfValue::parse(&content);
                let Some(map) = parsed.as_map() else {
                    continue;
                };
                let Some(appid) = map.get("appid").and_then(VdfValue::as_str) else {
                    continue;
                };
                let name = map
                    .get("name")
                    .and_then(VdfValue::as_str)
                    .unwrap_or("Nieznana gra")
                    .to_string();
                let install_dir = map
                    .get("installdir")
                    .and_then(VdfValue::as_str)
                    .map(|d| lib.join("common").join(d).to_string_lossy().into_owned());

                games.push(Game {
                    id: appid.to_string(),
                    title: name,
                    platform: Platform::Steam,
                    installed: true,
                    install_dir,
                    cover_path: find_cover_art(&root, appid),
                    playtime_minutes: None, // Steam nie trzyma tego lokalnie w prosty sposób — wymaga Web API + klucza.
                });
            }
        }
        Ok(games)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        self.launch_command_with_action(game_id, "rungameid")
    }
}

/// Pobiera czas gry (w minutach) dla wszystkich gier na koncie przez
/// oficjalne Steam Web API (`IPlayerService/GetOwnedGames`). Wymaga klucza
/// API (https://steamcommunity.com/dev/apikey) i SteamID64 — Steam nie
/// udostępnia tej informacji lokalnie w plikach `appmanifest_*.acf` w
/// żaden prosty sposób (tylko globalny, niepowiązany z konkretną grą
/// znacznik czasu ostatniego uruchomienia). Błędy sieciowe/braku
/// uprawnień są ciche — zwracana jest po prostu pusta mapa, żeby biblioteka
/// nadal się wyświetliła (bez czasu gry) zamiast się wywalić.
/// Dociąga opis i zrzuty ekranu z publicznego (nieuwierzytelnionego) Steam
/// Store API (`store.steampowered.com/api/appdetails`) — używane wyłącznie
/// do wzbogacenia widoku szczegółów gry, nigdy do logiki
/// instalacji/uruchamiania.
pub fn fetch_store_details(appid: &str) -> Option<super::GameDetails> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&l=polish");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    let entry = parsed.get(appid)?;
    if !entry.get("success")?.as_bool().unwrap_or(false) {
        return None;
    }
    let data = entry.get("data")?;

    let description = data
        .get("short_description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let screenshots = data
        .get("screenshots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("path_thumbnail").and_then(|p| p.as_str()))
                .map(String::from)
                .take(6)
                .collect()
        })
        .unwrap_or_default();

    Some(super::GameDetails { description, screenshots })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSearchResult {
    pub appid: u64,
    pub name: String,
    pub cover_url: Option<String>,
    pub price: Option<String>,
}

/// Przeszukuje katalog Steam przez publiczne, nieuwierzytelnione API
/// `store.steampowered.com/api/storesearch` — to jest realne "natywne"
/// przeglądanie sklepu (dane, nie strona WWW w webview), używane w
/// `Store.tsx`. Wyniki prowadzą do `steam://store/<appid>`, które otwiera
/// stronę produktu we wbudowanej przeglądarce klienta Steam — Hacker Mode
/// nie odtwarza całego procesu zakupu, tylko wyszukiwanie i przejście do
/// niego.
pub fn search_store(query: &str) -> Vec<StoreSearchResult> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc=pl&l=polish",
        urlencoding_minimal(query)
    );

    let result: Option<Vec<StoreSearchResult>> = (|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .ok()?;
        let response = client.get(&url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let parsed: serde_json::Value = response.json().ok()?;
        let items = parsed.get("items")?.as_array()?;
        Some(
            items
                .iter()
                .filter_map(|item| {
                    Some(StoreSearchResult {
                        appid: item.get("id")?.as_u64()?,
                        name: item.get("name")?.as_str()?.to_string(),
                        cover_url: item.get("tiny_image").and_then(|v| v.as_str()).map(String::from),
                        price: item
                            .get("price")
                            .and_then(|p| p.get("final_formatted"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                })
                .collect(),
        )
    })();

    result.unwrap_or_default()
}

/// Minimalne url-encode dla zapytania (spacje/znaki specjalne) — bez
/// dodawania zależności `urlencoding`, bo potrzebujemy tylko obsłużyć
/// zwykły tekst wyszukiwania.
fn urlencoding_minimal(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub fn fetch_owned_games_playtime(api_key: &str, steam_id64: &str) -> HashMap<String, u64> {
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={api_key}&steamid={steam_id64}&format=json&include_appinfo=false&include_played_free_games=true"
    );

    let result: Option<HashMap<String, u64>> = (|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .ok()?;
        let response = client.get(&url).send().ok()?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "Steam Web API zwróciło błąd (sprawdź klucz API / SteamID64 / czy profil jest publiczny)");
            return None;
        }
        let parsed: serde_json::Value = response.json().ok()?;
        let games = parsed.get("response")?.get("games")?.as_array()?;
        let mut map = HashMap::new();
        for game in games {
            let appid = game.get("appid")?.as_u64()?;
            let minutes = game.get("playtime_forever").and_then(|v| v.as_u64()).unwrap_or(0);
            map.insert(appid.to_string(), minutes);
        }
        Some(map)
    })();

    result.unwrap_or_default()
}

impl SteamProvider {
    /// Steam obsługuje kilka akcji przez protokół `steam://`:
    /// `rungameid/<id>` (uruchom), `install/<id>` (zainstaluj). Wspólna
    /// funkcja, żeby nie duplikować logiki wyboru `steam` vs Flatpak.
    fn launch_command_with_action(&self, game_id: &str, action: &str) -> Result<Command, StoreError> {
        if tool_available("steam") {
            let mut cmd = Command::new("steam");
            cmd.arg(format!("steam://{action}/{game_id}"));
            Ok(cmd)
        } else if tool_available("flatpak") {
            let mut cmd = Command::new("flatpak");
            cmd.args([
                "run",
                "com.valvesoftware.Steam",
                &format!("steam://{action}/{game_id}"),
            ]);
            Ok(cmd)
        } else {
            Err(StoreError::ToolNotInstalled("steam".into()))
        }
    }
}

/// Bardzo mały parser formatu VDF (Valve KeyValue) — wystarczający do
/// odczytu `libraryfolders.vdf` i `appmanifest_*.acf`. Format:
///
/// ```text
/// "AppState"
/// {
///     "appid"     "123"
///     "name"      "Przykładowa Gra"
///     "nested"
///     {
///         "a" "1"
///     }
/// }
/// ```
#[derive(Debug, Clone)]
enum VdfValue {
    Str(String),
    Map(HashMap<String, VdfValue>),
}

impl VdfValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            VdfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_map(&self) -> Option<&HashMap<String, VdfValue>> {
        match self {
            VdfValue::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Parsuje cały dokument VDF i zwraca korzeń jako `Map` (spłaszczając
    /// nazwę bloku najwyższego poziomu, np. `"AppState" { ... }` ->
    /// mapę z zawartością `{ ... }`).
    fn parse(input: &str) -> VdfValue {
        let tokens = tokenize(input);
        let mut pos = 0usize;
        // Pierwszy token to nazwa bloku najwyższego poziomu — pomijamy go
        // i parsujemy sam blok `{ ... }`.
        if tokens.len() >= 2 {
            pos = 1;
        }
        parse_block(&tokens, &mut pos)
    }
}

#[derive(Debug, Clone)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        s.push(chars[i + 1]);
                        i += 2;
                    } else {
                        s.push(chars[i]);
                        i += 1;
                    }
                }
                i += 1; // zamykający cudzysłów
                tokens.push(Token::Str(s));
            }
            '{' => {
                tokens.push(Token::Open);
                i += 1;
            }
            '}' => {
                tokens.push(Token::Close);
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ => {
                i += 1; // ignorujemy komentarze / nieobsługiwane znaki
            }
        }
    }
    tokens
}

fn parse_block(tokens: &[Token], pos: &mut usize) -> VdfValue {
    // Oczekujemy `Open` na start bloku; jeśli go nie ma (np. pusty plik),
    // zwracamy pustą mapę.
    if matches!(tokens.get(*pos), Some(Token::Open)) {
        *pos += 1;
    }
    let mut map = HashMap::new();
    while let Some(tok) = tokens.get(*pos) {
        match tok {
            Token::Close => {
                *pos += 1;
                break;
            }
            Token::Str(key) => {
                let key = key.clone();
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Str(value)) => {
                        map.insert(key, VdfValue::Str(value.clone()));
                        *pos += 1;
                    }
                    Some(Token::Open) => {
                        let nested = parse_block(tokens, pos);
                        map.insert(key, nested);
                    }
                    _ => {}
                }
            }
            Token::Open => {
                // Blok bez klucza — pomijamy (nieprawidłowy VDF).
                let _ = parse_block(tokens, pos);
            }
        }
    }
    VdfValue::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_acf() {
        let sample = r#"
        "AppState"
        {
            "appid"     "440"
            "name"      "Team Fortress 2"
            "installdir" "Team Fortress 2"
        }
        "#;
        let parsed = VdfValue::parse(sample);
        let map = parsed.as_map().unwrap();
        assert_eq!(map.get("appid").unwrap().as_str().unwrap(), "440");
        assert_eq!(
            map.get("name").unwrap().as_str().unwrap(),
            "Team Fortress 2"
        );
    }

    #[test]
    fn parses_nested_library_folders() {
        let sample = r#"
        "libraryfolders"
        {
            "0"
            {
                "path" "/mnt/games"
                "apps"
                {
                    "440" "12345"
                }
            }
        }
        "#;
        let parsed = VdfValue::parse(sample);
        let map = parsed.as_map().unwrap();
        let entry0 = map.get("0").unwrap().as_map().unwrap();
        assert_eq!(entry0.get("path").unwrap().as_str().unwrap(), "/mnt/games");
    }
}
