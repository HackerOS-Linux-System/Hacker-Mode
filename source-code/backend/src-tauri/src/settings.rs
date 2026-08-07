use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingTools {
    pub gamescope: bool,
    pub mangohud: bool,
    pub vkbasalt: bool,
}

impl Default for GamingTools {
    fn default() -> Self {
        Self {
            gamescope: false,
            mangohud: true,
            vkbasalt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    /// Odpowiednik zachowania z wersji 0.1: uruchomienie zewnętrznej
    /// aplikacji (Steam/Heroic/Lutris/gra) zamyka/chowa Hacker Mode, a po
    /// jej zamknięciu Hacker Mode wraca. Wyłączenie tego pozwala trzymać
    /// powłokę Hacker Mode działającą w tle (np. do szybkiego przełączania
    /// się overlayem).
    pub wrapper_mode_enabled: bool,
    pub gaming_tools: GamingTools,
    pub power_profile: String,
    /// Ostatnio wybrane platformy widoczne w bibliotece (filtr UI).
    pub enabled_platforms: Vec<String>,
    /// Opcjonalny klucz Steam Web API (https://steamcommunity.com/dev/apikey)
    /// i SteamID64 użytkownika — jeśli oba są ustawione, Hacker Mode
    /// dociąga realny czas gry dla biblioteki Steam. Bez nich
    /// `playtime_minutes` dla gier Steam zostaje `None` (Steam nie
    /// udostępnia tego lokalnie bez API). Wartości trzymane w zwykłym
    /// pliku konfiguracyjnym — jeśli w przyszłości Hacker Mode zacznie
    /// przechowywać inne sekrety, warto rozważyć trzymanie ich w
    /// keyring/secret-service zamiast zwykłego JSON-a.
    pub steam_api_key: Option<String>,
    pub steam_id64: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: "dark".into(),
            wrapper_mode_enabled: true,
            gaming_tools: GamingTools::default(),
            power_profile: "balanced".into(),
            enabled_platforms: vec![
                "steam".into(),
                "epic".into(),
                "gog".into(),
                "amazon".into(),
                "lutris".into(),
            ],
            steam_api_key: None,
            steam_id64: None,
        }
    }
}

fn default_language() -> String {
    std::env::var("LANG")
        .ok()
        .and_then(|l| l.split('_').next().map(str::to_string))
        .filter(|l| l == "pl")
        .unwrap_or_else(|| "en".to_string())
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hacker-mode")
        .join("config.json")
}

impl Settings {
    pub fn load_or_default() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|err| {
                tracing::warn!(%err, "Nieprawidłowy config.json, używam ustawień domyślnych");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
