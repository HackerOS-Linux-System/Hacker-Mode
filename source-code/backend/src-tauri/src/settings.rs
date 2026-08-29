use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    ///
    /// `#[serde(default)]`: `Vec<T>` (w przeciwieństwie do `Option<T>`) NIE
    /// jest przez serde automatycznie traktowany jako pusty przy braku
    /// pola w JSON-ie — bez tej adnotacji `config.json` zapisany PRZED
    /// dodaniem tego pola (wcześniejsze wersje Hacker Mode) w ogóle nie
    /// dałby się wczytać, co i tak "leczyło się samo" przez
    /// `load_or_default` (całkowity powrót do ustawień domyślnych przy
    /// błędzie parsowania) — ale kosztem utraty WSZYSTKICH innych ustawień
    /// użytkownika (motywu, klucza Steam API, itd.), nie tylko tego pola.
    #[serde(default)]
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
    /// Ręczne nadpisanie ścieżki do prefiksu Wine z zainstalowanym EA app —
    /// używane tylko, gdy autodetekcja (`ea_cli::find_prefix`, sprawdzająca
    /// domyślne lokalizacje Lutrisa) nic nie znajdzie, np. bo użytkownik
    /// wybrał przy instalacji nietypowy katalog. `None` = poleganie
    /// wyłącznie na autodetekcji. Patrz `commands/stores/ea.rs`.
    pub ea_wine_prefix: Option<String>,
    /// To samo co `ea_wine_prefix`, ale dla Battle.net — patrz
    /// `commands/stores/battlenet.rs`.
    pub battlenet_wine_prefix: Option<String>,
    /// Ręczne nadpisania okładek gier, klucz `"<platforma>:<id_gry>"` (np.
    /// `"steam:440"`) → URL (http/https) albo lokalna ścieżka pliku
    /// obrazka. Ustawiane przez `commands::set_game_cover`
    /// (`GameDetail.tsx` → "Zmień okładkę") i aplikowane na końcu
    /// `commands::list_games`, PO zbudowaniu pełnej biblioteki przez
    /// wszystkich providerów — więc działa identycznie dla gry z lokalną
    /// okładką z cache (Steam/GOG/Epic/Amazon/Lutris), zdalnym URL-em
    /// (Steam CDN dla gier tylko posiadanych) i brakiem okładki (Lutris
    /// bez lokalnego bannera, EA, Battle.net).
    #[serde(default)]
    pub cover_overrides: HashMap<String, String>,
    /// Opcjonalny klucz API SteamGridDB (https://www.steamgriddb.com/profile/preferences/api)
    /// — zapasowe źródło okładek dla platform bez żadnego innego dostępu
    /// do artworku (dziś: EA app, Battle.net — patrz `steamgriddb.rs` i
    /// `commands/stores/{ea,battlenet}.rs`). `None`/puste = wyłączone,
    /// dokładnie jak brak `steam_api_key` wyżej. `#[serde(default)]` z
    /// tego samego powodu co `cover_overrides` — pole dodane już PO
    /// pierwszych wydaniach `config.json`.
    #[serde(default)]
    pub steamgriddb_api_key: Option<String>,
    /// Własne tagi użytkownika per gra (np. "Ulubione", "Kooperacja", "Do
    /// ogrania") — klucz w tym samym formacie co `cover_overrides`
    /// (`"<platform-slug>:<game-id>"`, patrz `cover_override_key`).
    /// Wyłącznie do filtrowania/organizacji biblioteki po stronie
    /// frontendu (`Library.tsx`) — Hacker Mode nie interpretuje żadnego
    /// tagu w żaden szczególny sposób.
    #[serde(default)]
    pub game_tags: HashMap<String, Vec<String>>,
    /// Katalog na kopie zapasowe zapisów gier (patrz `cloud_saves.rs`) —
    /// `None` = funkcja wyłączona (przyciski "Utwórz kopię" w
    /// `GameDetail.tsx` pokazują wtedy komunikat, że trzeba go najpierw
    /// ustawić w Ustawieniach). Jeśli ten katalog jest zsynchronizowany
    /// przez zewnętrzny klient chmury (Nextcloud/Dropbox/Syncthing),
    /// kopie faktycznie trafiają "do chmury" — ale to synchronizacja
    /// PLIKÓW przez program użytkownika, nie integracja Hacker Mode z
    /// żadnym konkretnym dostawcą (patrz moduł-dokumentacja `cloud_saves.rs`).
    #[serde(default)]
    pub cloud_saves_backup_dir: Option<String>,
    /// Ręcznie wskazany katalog zapisu per gra — klucz w tym samym
    /// formacie co `game_tags`/`cover_overrides`. Świadomie NIE
    /// wykrywane automatycznie, patrz moduł-dokumentacja `cloud_saves.rs`.
    #[serde(default)]
    pub game_save_paths: HashMap<String, String>,
    /// Gotowy string `SDL_GAMECONTROLLERCONFIG` per gra (wygenerowany
    /// zewnętrznym narzędziem, np. "SDL2 Gamepad Tool") — patrz
    /// moduł-dokumentacja `controllers.rs`. Klucz jak wyżej.
    #[serde(default)]
    pub game_controller_configs: HashMap<String, String>,
    /// Globalny prefiks polecenia, którym owija się KAŻDE uruchomienie
    /// gry (np. `"gamemoderun"`, `"prime-run"`, `"strace -f"`) — dzielony
    /// na słowa prostym `split_whitespace` (patrz
    /// `launcher::wrap_with_custom_prefix`), bez pełnego parsera powłoki
    /// (cudzysłowy/zmienne w środku nie są obsługiwane — to świadome
    /// uproszczenie dla typowego przypadku "jedna komenda, zero
    /// argumentów ze spacjami w środku"). Nakładany jako NAJBARDZIEJ
    /// zewnętrzna warstwa, na zewnątrz `gamescope` (jeśli też włączony) —
    /// więc np. `"gamemoderun"` + gamescope daje efektywnie
    /// `gamemoderun gamescope -- <gra>`.
    #[serde(default)]
    pub custom_launch_prefix: Option<String>,
    #[serde(default)]
    pub notifications: NotificationSettings,
}

/// Granularna kontrola nad powiadomieniami systemowymi (patrz
/// `notifications.rs`) — dodane po tym, jak kilka kolejnych funkcji
/// (instalacja, automatyczna kopia zapasowa) zaczęło je wysyłać bez
/// żadnej możliwości wyłączenia, co dla kogoś bez środowiska
/// obsługującego Desktop Notifications Specification (rzadkie, ale się
/// zdarza) albo kogoś, kto po prostu ich nie chce, byłoby wyłącznie
/// irytujące. Domyślnie wszystkie włączone (zachowanie sprzed dodania
/// tego ustawienia).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(default = "default_true")]
    pub on_install: bool,
    #[serde(default = "default_true")]
    pub on_game_exit: bool,
    #[serde(default = "default_true")]
    pub on_backup_error: bool,
}

fn default_true() -> bool {
    true
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self { on_install: true, on_game_exit: true, on_backup_error: true }
    }
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
                "ea".into(),
                "battlenet".into(),
            ],
            steam_api_key: None,
            steam_id64: None,
            ea_wine_prefix: None,
            battlenet_wine_prefix: None,
            cover_overrides: HashMap::new(),
            steamgriddb_api_key: None,
            game_tags: HashMap::new(),
            cloud_saves_backup_dir: None,
            game_save_paths: HashMap::new(),
            game_controller_configs: HashMap::new(),
            custom_launch_prefix: None,
            notifications: NotificationSettings::default(),
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
