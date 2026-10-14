use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Kompozytor Wayland, na którym działa sesja Hacker Mode.
///
/// Domyślnie `labwc` (dojrzały kompozytor wlroots z natywnym backendem
/// DRM/KMS, więc sesja działa także z gołego TTY). `comphwde` (projekt
/// HWDE, sterowany przez IPC) włącza się przez `"compositor": "comphwde"`
/// w `~/.config/Hacker-Mode/config.json` albo zmienną środowiskową
/// `HACKER_MODE_COMPOSITOR`. Wartość czyta `scripts/hacker-mode-session`
/// PRZED startem sesji - zmiana wymaga ponownego zalogowania.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compositor {
    #[default]
    Labwc,
    Comphwde,
}

impl Compositor {
    pub fn as_str(self) -> &'static str {
        match self {
            Compositor::Labwc => "labwc",
            Compositor::Comphwde => "comphwde",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "labwc" => Some(Compositor::Labwc),
            "comphwde" => Some(Compositor::Comphwde),
            _ => None,
        }
    }
}

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
#[serde(default)]
pub struct Settings {
    /// Kompozytor sesji - patrz [`Compositor`]. Pole jest częścią tego
    /// samego pliku `config.json` co reszta ustawień.
    pub compositor: Compositor,
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
    /// Próg (w sekundach), poniżej którego zamknięcie gry jest
    /// traktowane jako prawdopodobny crash zamiast normalnego wyjścia —
    /// używany zarówno przy PID-owym śledzeniu procesu (patrz
    /// `launcher::run_wrapped`), jak i przy śledzeniu Steam przez Web API
    /// (patrz `launcher::finish_steam_web_api_session`). Wcześniej była
    /// to stała `8` wpisana na sztywno NIEZALEŻNIE w trzech miejscach
    /// (`launcher.rs`, `GameCard.tsx`, `GameDetail.tsx`, patrz README
    /// „Do zrobienia dalej” — ta sama wartość, ale trzy osobne źródła
    /// prawdy, które trzeba było pamiętać zmieniać razem) — teraz jest to
    /// jedno, konfigurowalne ustawienie (patrz panel „Zaawansowane:
    /// uruchamianie” w `Settings.tsx` i komenda
    /// `commands::set_crash_detection_threshold`), które frontend czyta
    /// z tego samego obiektu `Settings` co resztę ustawień zamiast
    /// trzymać własną kopię liczby.
    #[serde(default = "default_crash_detection_threshold_seconds")]
    pub crash_detection_threshold_seconds: u64,
    /// Domyślnie `false`: Hacker Mode ODMAWIA modyfikacji plików
    /// konfiguracyjnych Steama/Lutrisa (`config.vdf`, YAML gry), gdy dany
    /// klient działa - Steam i tak nadpisze plik przy zamknięciu, a
    /// równoległy zapis grozi jego uszkodzeniem. Ustaw `true`, jeśli
    /// świadomie chcesz zapisywać mimo to.
    #[serde(default)]
    pub allow_config_write_while_running: bool,
    /// Domyślnie `false`: odinstalowanie gry Lutrisa tylko oznacza wpis w
    /// `pga.db` jako niezainstalowany. `true` dodatkowo kasuje katalog gry
    /// (tylko jeśli leży w bezpiecznej lokalizacji - patrz
    /// `commands/stores/lutris.rs`).
    #[serde(default)]
    pub lutris_delete_files_on_uninstall: bool,
}

fn default_crash_detection_threshold_seconds() -> u64 {
    8
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
            compositor: Compositor::default(),
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
            crash_detection_threshold_seconds: default_crash_detection_threshold_seconds(),
            allow_config_write_while_running: false,
            lutris_delete_files_on_uninstall: false,
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

fn config_root() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// `~/.config/Hacker-Mode/config.json` (od v1.1).
pub fn config_path() -> PathBuf {
    config_root().join("Hacker-Mode").join("config.json")
}

/// `~/.config/hacker-mode/config.json` - lokalizacja sprzed v1.1; czytana
/// tylko jednorazowo przy migracji.
fn legacy_config_path() -> PathBuf {
    config_root().join("hacker-mode").join("config.json")
}

impl Settings {
    /// Wczytuje ustawienia z `~/.config/Hacker-Mode/config.json`; jeśli go
    /// nie ma, a istnieje stary plik z `~/.config/hacker-mode/`, migruje go
    /// (stary zostaje nietknięty). Uszkodzony plik NIE jest po cichu
    /// nadpisywany: najpierw trafia do `config.json.corrupt-<unix-time>`.
    pub fn load_or_default() -> Self {
        Self::load_from(&config_path(), Some(&legacy_config_path()))
    }

    pub fn load_from(path: &Path, legacy: Option<&Path>) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Settings>(&content) {
                Ok(settings) => settings,
                Err(err) => {
                    tracing::warn!(%err, "Nieprawidłowy config.json, używam ustawień domyślnych (oryginał zachowany w kopii)");
                    Self::quarantine_corrupt(path);
                    Settings::default()
                }
            },
            Err(_) => {
                if let Some(legacy_path) = legacy {
                    if let Ok(content) = std::fs::read_to_string(legacy_path) {
                        if let Ok(migrated) = serde_json::from_str::<Settings>(&content) {
                            tracing::info!(from = %legacy_path.display(), to = %path.display(), "Migracja ustawień do nowej lokalizacji");
                            if let Err(err) = migrated.save_to(path) {
                                tracing::warn!(%err, "Nie udało się zapisać zmigrowanych ustawień");
                            }
                            return migrated;
                        }
                    }
                }
                Settings::default()
            }
        }
    }

    fn quarantine_corrupt(path: &Path) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut backup = path.as_os_str().to_owned();
        backup.push(format!(".corrupt-{stamp}"));
        if let Err(err) = std::fs::rename(path, PathBuf::from(backup)) {
            tracing::warn!(%err, "Nie udało się zachować uszkodzonego config.json");
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&config_path())
    }

    /// Zapis atomowy (plik tymczasowy + `rename`) z uprawnieniami `0600`
    /// (plik zawiera klucze API) w katalogu `0700`.
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
        let json = serde_json::to_string_pretty(self)?;
        crate::util::write_atomic(path, json.as_bytes(), Some(0o600))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_compositor_is_labwc() {
        assert_eq!(Settings::default().compositor, Compositor::Labwc);
        assert_eq!(Compositor::default().as_str(), "labwc");
    }

    #[test]
    fn compositor_parses_and_serializes_lowercase() {
        assert_eq!(Compositor::parse("COMPHWDE"), Some(Compositor::Comphwde));
        assert_eq!(Compositor::parse("sway"), None);
        let json = serde_json::to_string(&Compositor::Comphwde).unwrap();
        assert_eq!(json, "\"comphwde\"");
    }

    #[test]
    fn partial_config_keeps_defaults_instead_of_failing() {
        // Plik zawiera tylko jedno pole - reszta musi dostać wartości domyślne
        // (wcześniej brak np. `language` unieważniał CAŁY plik).
        let s: Settings = serde_json::from_str(r#"{"compositor":"comphwde","theme":"light"}"#).unwrap();
        assert_eq!(s.compositor, Compositor::Comphwde);
        assert_eq!(s.theme, "light");
        assert_eq!(s.crash_detection_threshold_seconds, 8);
        assert!(s.wrapper_mode_enabled);
    }

    #[test]
    fn save_and_load_roundtrip_with_private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Hacker-Mode").join("config.json");
        let mut s = Settings::default();
        s.compositor = Compositor::Comphwde;
        s.steam_api_key = Some("SECRET".into());
        s.save_to(&path).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config z kluczami API musi być prywatny");
        let dir_mode = std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700);

        let loaded = Settings::load_from(&path, None);
        assert_eq!(loaded.compositor, Compositor::Comphwde);
        assert_eq!(loaded.steam_api_key.as_deref(), Some("SECRET"));
    }

    #[test]
    fn legacy_config_is_migrated_and_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("hacker-mode").join("config.json");
        let new = dir.path().join("Hacker-Mode").join("config.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, r#"{"theme":"light"}"#).unwrap();

        let loaded = Settings::load_from(&new, Some(&legacy));
        assert_eq!(loaded.theme, "light");
        assert!(new.exists(), "zmigrowany plik powinien zostać zapisany w nowej lokalizacji");
        assert!(legacy.exists(), "stary plik nie może być usunięty");
        // Drugi odczyt korzysta już z nowego pliku.
        assert_eq!(Settings::load_from(&new, None).theme, "light");
    }

    #[test]
    fn corrupt_config_is_quarantined_not_destroyed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{ to nie jest json").unwrap();
        let loaded = Settings::load_from(&path, None);
        assert_eq!(loaded.theme, "dark");
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("config.json.corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), "{ to nie jest json");
    }
}
