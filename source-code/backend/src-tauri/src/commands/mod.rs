pub mod stores;

use tauri::{AppHandle, State};

use crate::settings::Settings;
use crate::state::AppState;
use crate::system::{self, ActionResult};
use stores::Platform;

// --- Biblioteka gier -----------------------------------------------------

/// Buduje parę `(klucz Steam Web API, SteamID64)` potrzebną do pobrania
/// pełnej biblioteki (nie tylko zainstalowanych gier — patrz
/// `stores::list_all_games`), z automatycznym uzupełnieniem SteamID64,
/// gdy nie jest ustawiony ręcznie w ustawieniach.
///
/// UX-owy powód tej funkcji: klucz API trzeba i tak wkleić ręcznie (to
/// prywatny sekret, generowany na steamcommunity.com/dev/apikey — nie da
/// się go w żaden sposób wykryć lokalnie), ale SteamID64 to publiczna
/// informacja, którą sam Steam już zna i trzyma lokalnie w
/// `config/loginusers.vdf` (patrz `steam::SteamProvider::find_local_steamid64`).
/// Wcześniej użytkownik musiał go ręcznie znaleźć przez zewnętrzne
/// narzędzia (np. steamid.io) — teraz wystarczy sam klucz API, a
/// SteamID64 dogaduje się sam, o ile Steam jest zainstalowany i
/// zalogowany na tej maszynie. Ręczne ustawienie w Ustawieniach nadal ma
/// pierwszeństwo (np. dla kogoś, kto chce pociągnąć bibliotekę innego
/// konta niż to zalogowane lokalnie).
fn build_steam_credentials(api_key: Option<String>, manual_steam_id64: Option<String>) -> Option<(String, String)> {
    let api_key = api_key.filter(|k| !k.is_empty())?;

    if let Some(id) = manual_steam_id64.filter(|i| !i.is_empty()) {
        return Some((api_key, id));
    }

    match stores::steam::SteamProvider::find_local_steamid64() {
        Some(id) => {
            tracing::info!(steam_id64 = %id, "Steam: SteamID64 wykryty automatycznie z loginusers.vdf");
            Some((api_key, id))
        }
        None => {
            // Klucz API jest ustawiony, ale nie udało się ani ręcznie, ani
            // automatycznie ustalić SteamID64 — funkcja "posiadane, ale
            // niezainstalowane gry" po prostu się nie uruchomi. To
            // najczęstsze miejsce do sprawdzenia, gdyby ktoś zgłosił "mam
            // klucz API, a i tak nie widzę pełnej biblioteki".
            tracing::warn!(
                "Steam: ustawiono klucz API, ale nie udało się ustalić SteamID64 (ani ręcznie w ustawieniach, ani automatycznie z loginusers.vdf) — pomijam dociąganie posiadanych gier"
            );
            None
        }
    }
}

#[tauri::command]
pub fn list_games(state: State<AppState>) -> stores::LibraryLoadResult {
    let settings = state.settings.lock().unwrap();
    let steam_api_key = settings.steam_api_key.clone();
    let steam_id64_setting = settings.steam_id64.clone();
    let enabled_platforms = settings.enabled_platforms.clone();
    let cover_overrides = settings.cover_overrides.clone();
    drop(settings);

    let credentials = build_steam_credentials(steam_api_key, steam_id64_setting);
    let mut result = stores::list_all_games(credentials.as_ref().map(|(k, i)| (k.as_str(), i.as_str())));

    // BUGFIX: `enabled_platforms` było zadeklarowane w `Settings` (i miało
    // domyślnie wszystkie platformy zaznaczone) od samego początku, ale
    // nic go nigdy nie odczytywało — biblioteka zawsze pokazywała
    // wszystko, niezależnie od tego, co użytkownik wybrałby w ustawieniach.
    // Pusta lista traktowana jest jak "bez filtra" (pokaż wszystko) — to
    // ochrona przed przypadkowym całkowitym wyczyszczeniem biblioteki
    // przez stary/uszkodzony `config.json` sprzed tej zmiany, w którym to
    // pole mogło nigdy nie zostać zapisane.
    if !enabled_platforms.is_empty() {
        result.games.retain(|g| {
            enabled_platforms
                .iter()
                .any(|p| p.eq_ignore_ascii_case(g.platform.slug()))
        });
    }

    if !cover_overrides.is_empty() {
        for game in result.games.iter_mut() {
            if let Some(custom) = cover_overrides.get(&cover_override_key(game.platform, &game.id)) {
                game.cover_path = Some(custom.clone());
            }
        }
    }

    // Uzupełnia `playtime_minutes` lokalnym licznikiem Hacker Mode
    // (`playtime.rs`) TYLKO tam, gdzie provider platformy nic nie zwrócił
    // — patrz moduł-dokumentacja `playtime.rs`. Robione tu, centralnie,
    // na PEŁNEJ liście (po scaleniu wszystkich platform i gier
    // posiadanych-ale-niezainstalowanych wyżej), żeby nie czytać
    // `playtime.json` osobno w każdym providerze.
    crate::playtime::merge_into(&mut result.games);

    result
}

/// Klucz w `Settings::cover_overrides` dla danej gry — wydzielone do
/// osobnej funkcji, żeby `list_games` i `set_game_cover` nie mogły się
/// kiedyś rozjechać co do formatu klucza.
/// Klucz w `Settings::cover_overrides`/`game_tags`/`game_save_paths`/
/// `game_controller_configs` dla danej gry — `pub(crate)`, bo poza
/// `commands/mod.rs` używa go też `launcher.rs` (do odczytania
/// `game_controller_configs` przy uruchamianiu gry).
pub(crate) fn cover_override_key(platform: Platform, game_id: &str) -> String {
    format!("{}:{game_id}", platform.slug())
}

/// Ustawia (albo, gdy `cover_url` to `None`, usuwa) ręczne nadpisanie
/// okładki dla danej gry — patrz `Settings::cover_overrides`. Przyjmuje
/// zarówno URL (http/https, np. wklejony link do grafiki), jak i ścieżkę
/// do lokalnego pliku na dysku (podobnie jak `cover_path` budowane przez
/// samych providerów).
///
/// Dla lokalnych ścieżek sprawdzamy tu TYLKO, czy plik w ogóle istnieje —
/// to nie jest pełna walidacja (nie odtwarzamy dopasowania wzorców glob z
/// `app.security.assetProtocol.scope` w `tauri.conf.json`, które i tak
/// egzekwuje sam webview Tauri w czasie renderowania, patrz
/// `resolveCoverSrc` we froncie). Plik może więc istnieć, przejść tę
/// walidację, a mimo to nie wyrenderować się, jeśli leży POZA dozwolonym
/// scope (np. plik z Pulpitu zamiast `~/Games`/cache) — to wyłapuje
/// dopiero front (patrz `GameDetail.tsx::saveCover`, które faktycznie
/// próbuje załadować obrazek po zapisaniu i pokazuje komunikat, jeśli się
/// nie uda). Ta walidacja tutaj łapie za to najczęstszy przypadek: zwykłą
/// literówkę w ścieżce.
#[tauri::command]
pub fn set_game_cover(state: State<AppState>, platform: Platform, game_id: String, cover_url: Option<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    let key = cover_override_key(platform, &game_id);
    match cover_url {
        Some(url) if !url.trim().is_empty() => {
            let url = url.trim().to_string();
            let is_remote = url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:");
            if !is_remote && !std::path::Path::new(&url).is_file() {
                return Err(format!("Plik nie istnieje: {url}"));
            }
            settings.cover_overrides.insert(key, url);
        }
        _ => {
            settings.cover_overrides.remove(&key);
        }
    }
    settings.save().map_err(|e| e.to_string())
}

/// Ustawia (nadpisując w całości) listę własnych tagów dla danej gry —
/// patrz `Settings::game_tags`. Celowo NADPISUJE, nie dokłada: frontend
/// (`GameDetail.tsx`) zawsze wysyła pełną, już zmienioną listę, tak samo
/// jak `enabled_platforms`/`gaming_tools` gdzie indziej w tym pliku —
/// zamiast osobnych komend `add_tag`/`remove_tag`, które musiałyby się
/// martwić o duplikaty/kolejność.
///
/// Tagi (w przeciwieństwie do `cover_overrides`) NIE są mergowane do
/// struktury `Game` w `list_games` — frontend czyta je bezpośrednio z
/// `Settings::game_tags` (zwracanego przez `get_settings`) i sam
/// dopasowuje po tym samym kluczu (`cover_override_key`), żeby dodanie
/// tego pola nie wymagało dotykania literałów `Game { ... }` w KAŻDYM z
/// siedmiu providerów sklepów.
#[tauri::command]
pub fn set_game_tags(state: State<AppState>, platform: Platform, game_id: String, tags: Vec<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    let key = cover_override_key(platform, &game_id);
    let cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if cleaned.is_empty() {
        settings.game_tags.remove(&key);
    } else {
        settings.game_tags.insert(key, cleaned);
    }
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_game(
    app: AppHandle,
    state: State<AppState>,
    platform: Platform,
    game_id: String,
) -> Result<(), String> {
    crate::launcher::launch_game(app, state, platform, game_id)
}

#[tauri::command]
pub fn stop_game(state: State<AppState>, platform: Platform, game_id: String, force: bool) -> Result<(), String> {
    crate::launcher::stop_game(&state, platform, &game_id, force)
}

#[tauri::command]
pub fn is_game_running(state: State<AppState>, platform: Platform, game_id: String) -> bool {
    crate::launcher::is_game_running(&state, platform, &game_id)
}

#[tauri::command]
pub fn launch_store_client(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<(), String> {
    crate::launcher::launch_store_client(app, state, &name)
}

#[tauri::command]
pub fn restart_apps(app: AppHandle) -> crate::launcher::ActionSummary {
    crate::launcher::restart_apps(&app)
}

#[tauri::command]
pub fn install_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    crate::launcher::install_game(app, platform, game_id)
}

#[tauri::command]
pub fn uninstall_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    crate::launcher::uninstall_game(app, platform, game_id)
}

#[tauri::command]
pub fn fetch_game_details(platform: Platform, game_id: String) -> Option<stores::GameDetails> {
    stores::fetch_game_details(platform, &game_id)
}

#[tauri::command]
pub fn search_steam_store(query: String) -> Vec<stores::steam::StoreSearchResult> {
    stores::steam::search_store(&query)
}

#[tauri::command]
pub fn open_steam_store_page(appid: u64) -> Result<(), String> {
    std::process::Command::new("steam")
        .arg(format!("steam://store/{appid}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Odpowiednik `search_steam_store`, ale dla katalogu GOG — patrz
/// `stores::gog::search_store` po pełny opis endpointu.
#[tauri::command]
pub fn search_gog_store(query: String) -> Vec<stores::gog::StoreSearchResult> {
    stores::gog::search_store(&query)
}

/// Odpowiednik `open_steam_store_page`, ale dla GOG — w przeciwieństwie do
/// Steam, GOG nie ma na Linuksie działającego klienta z własnym schematem
/// URL (`gog://` nie jest niczym obsługiwane poza Windowsem/GOG Galaxy),
/// więc otwieramy zwykłą stronę produktu w domyślnej przeglądarce przez
/// `xdg-open` — dokładnie tak samo jak `store_login::open_login_window`
/// otwiera zewnętrzne odnośniki dla platform bez wbudowanego webview.
#[tauri::command]
pub fn open_gog_store_page(slug: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(format!("https://www.gog.com/game/{slug}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn store_is_logged_in(platform: Platform) -> bool {
    crate::launcher::is_logged_in(platform)
}

#[tauri::command]
pub fn store_login_start(platform: Platform) -> Result<stores::LoginFlow, String> {
    crate::launcher::login_start(platform)
}

#[tauri::command]
pub fn store_login_submit(platform: Platform, code: String) -> Result<(), String> {
    crate::launcher::login_submit(platform, code)
}

#[tauri::command]
pub fn open_store_login_window(app: AppHandle, platform: Platform) -> Result<(), String> {
    crate::store_login::open_login_window(app, platform)
}

#[tauri::command]
pub fn submit_store_login_code(app: AppHandle, platform: Platform, code: String) -> Result<(), String> {
    crate::store_login::submit_login_code(app, platform, code)
}

// --- Ustawienia -----------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn is_dev_mode(state: State<AppState>) -> bool {
    state.is_dev()
}

// --- Akcje systemowe -------------------------------------------------------

#[tauri::command]
pub fn audio_action(action: String) -> ActionResult {
    match action.as_str() {
        "increase" => system::audio_volume_up(),
        "decrease" => system::audio_volume_down(),
        "toggle_mute" => system::audio_toggle_mute(),
        other => ActionResult { ok: false, message: format!("Nieznana akcja audio: {other}") },
    }
}

#[tauri::command]
pub fn display_action(action: String) -> ActionResult {
    match action.as_str() {
        "increase" => system::display_brightness_up(),
        "decrease" => system::display_brightness_down(),
        other => ActionResult { ok: false, message: format!("Nieznana akcja ekranu: {other}") },
    }
}

#[tauri::command]
pub fn network_action(action: String, ssid: Option<String>, password: Option<String>) -> ActionResult {
    match action.as_str() {
        "toggle_wifi" => system::network_toggle_wifi(),
        "list_wifi" => system::network_list_wifi(),
        "connect_wifi" => match ssid {
            Some(ssid) => system::network_connect_wifi(&ssid, password.as_deref()),
            None => ActionResult { ok: false, message: "Brak SSID".into() },
        },
        other => ActionResult { ok: false, message: format!("Nieznana akcja sieci: {other}") },
    }
}

#[tauri::command]
pub fn bluetooth_action(action: String, mac_address: Option<String>) -> ActionResult {
    match action.as_str() {
        "scan" => system::bluetooth_scan(),
        "pair" => match mac_address {
            Some(mac) => system::bluetooth_pair(&mac),
            None => ActionResult { ok: false, message: "Brak adresu MAC".into() },
        },
        other => ActionResult { ok: false, message: format!("Nieznana akcja bluetooth: {other}") },
    }
}

#[tauri::command]
pub fn power_action(action: String, state: State<AppState>) -> ActionResult {
    if state.is_dev() && matches!(action.as_str(), "shutdown" | "restart" | "sleep") {
        return ActionResult {
            ok: false,
            message: "Akcje zasilania są zablokowane w trybie `dev`.".into(),
        };
    }
    match action.as_str() {
        "power_saving" | "balanced" | "performance" => system::power_set_profile(&action),
        "shutdown" => system::power_shutdown(),
        "restart" => system::power_restart(),
        "sleep" => system::power_sleep(),
        other => ActionResult { ok: false, message: format!("Nieznana akcja zasilania: {other}") },
    }
}

#[tauri::command]
pub fn switch_to_desktop_session(app: AppHandle, session: String) -> Result<(), String> {
    // Odpowiednik "switchToPlasma" z wersji 0.1, uogólniony na dowolną
    // sesję desktopową skonfigurowaną w menedżerze logowania.
    let command = match session.as_str() {
        "plasma" => "startplasma-wayland",
        "gnome" => "gnome-session",
        other => return Err(format!("Nieobsługiwana sesja: {other}")),
    };
    std::process::Command::new(command)
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

/// Ocena kompatybilności ProtonDB dla gry Steam — patrz moduł-dokumentacja
/// `protondb.rs`. Tylko Steam ma sens (ProtonDB nie zna żadnej innej
/// platformy) — dla pozostałych platform ta komenda po prostu nigdy nie
/// jest wołana przez frontend (`GameDetail.tsx` pokazuje plakietkę
/// wyłącznie dla `platform === "steam"`), więc nie ma tu osobnej walidacji
/// platformy — to nie jest ścieżka bezpieczeństwa, tylko dobór treści UI.
///
/// Cache'owane na dobę (`net_cache`, patrz jego moduł-dokumentacja) —
/// oceny ProtonDB praktycznie nigdy nie zmieniają się w skali minut, więc
/// każde ponowne wejście w szczegóły tej samej gry w tym oknie czasu nie
/// generuje kolejnego zapytania do `protondb.com`.
#[tauri::command]
pub fn fetch_protondb_rating(appid: u64) -> Option<crate::protondb::ProtonDbSummary> {
    const TTL_SECONDS: u64 = 60 * 60 * 24;
    crate::net_cache::get_or_fetch("protondb.json", &appid.to_string(), TTL_SECONDS, || {
        crate::protondb::fetch_rating(appid)
    })
}

/// Lista osiągnięć gracza dla danej gry Steam — patrz moduł-dokumentacja
/// `steam::fetch_achievements`. Wymaga tych samych danych logowania co
/// czas gry Steam (`Settings::steam_api_key`/`steam_id64`); zwraca pustą
/// listę (nie błąd), jeśli którejś brakuje — frontend wtedy po prostu nie
/// pokazuje sekcji osiągnięć zamiast pokazywać komunikat błędu na każdej
/// karcie gry Steam, dopóki użytkownik nie skonfiguruje klucza.
///
/// Cache'owane KRÓCEJ niż ProtonDB (kilkanaście minut, nie doba) — patrz
/// moduł-dokumentacja `net_cache.rs` po wyjaśnienie, dlaczego (postęp
/// gracza w tym wyniku realnie się zmienia). Klucz cache'a łączy appid I
/// SteamID64 (`"<steamid64>:<appid>"`), żeby przełączenie się na inne
/// konto (`Settings::steam_id64`) nie pokazywało po cichu osiągnięć
/// poprzedniego konta z cache'a.
#[tauri::command]
pub fn fetch_steam_achievements(state: State<AppState>, appid: String) -> Vec<stores::steam::Achievement> {
    const TTL_SECONDS: u64 = 60 * 15;
    let settings = state.settings.lock().unwrap();
    let (Some(api_key), Some(steam_id64)) = (settings.steam_api_key.clone(), settings.steam_id64.clone()) else {
        return Vec::new();
    };
    drop(settings);
    let cache_key = format!("{steam_id64}:{appid}");
    crate::net_cache::get_or_fetch("achievements.json", &cache_key, TTL_SECONDS, || {
        let list = stores::steam::fetch_achievements(&api_key, &steam_id64, &appid);
        if list.is_empty() {
            None
        } else {
            Some(list)
        }
    })
    .unwrap_or_default()
}

/// Opcje wersji Proton/Wine dostępne do przypisania danej grze — patrz
/// moduł-dokumentacja `compat_tools.rs`. Zwraca też AKTUALNIE przypisaną
/// wersję (`current`), żeby frontend mógł od razu zaznaczyć właściwą
/// opcję w rozwijanej liście bez osobnego zapytania.
#[derive(Debug, serde::Serialize)]
pub struct CompatToolOptions {
    /// Nazwy do wyboru (dla Steam: `internal_name` z
    /// `compat_tools::CompatToolOption`, patrz jego moduł-dokumentacja po
    /// wyjaśnienie, dlaczego to tylko podzbiór wszystkich zainstalowanych
    /// wersji Protona; dla Lutris: nazwy katalogów w
    /// `~/.local/share/lutris/runners/wine/`).
    pub options: Vec<CompatToolOptionDto>,
    pub current: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CompatToolOptionDto {
    pub value: String,
    pub label: String,
}

#[tauri::command]
pub fn get_compat_tool_options(platform: Platform, game_id: String) -> CompatToolOptions {
    match platform {
        Platform::Steam => {
            let options = crate::compat_tools::list_settable_proton_versions()
                .into_iter()
                .map(|o| CompatToolOptionDto { value: o.internal_name, label: o.display_name })
                .collect();
            CompatToolOptions { options, current: crate::compat_tools::get_steam_compat_tool(&game_id) }
        }
        Platform::Lutris => {
            let options = crate::compat_tools::list_lutris_wine_versions()
                .into_iter()
                .map(|v| CompatToolOptionDto { value: v.clone(), label: v })
                .collect();
            CompatToolOptions { options, current: crate::compat_tools::get_lutris_wine_version(&game_id) }
        }
        _ => CompatToolOptions { options: Vec::new(), current: None },
    }
}

/// Ustawia wersję Proton (Steam)/Wine (Lutris) dla danej gry — patrz
/// `compat_tools::set_steam_compat_tool`/`set_lutris_wine_version` po
/// pełny opis mechanizmu i jego ograniczeń. `Ok(Some(ostrzeżenie))`
/// oznacza, że zapis się udał, ale odpowiedni klient (Steam/Lutris) był w
/// trakcie uruchomiony — frontend powinien to pokazać jako ostrzeżenie,
/// nie błąd.
#[tauri::command]
pub fn set_compat_tool(platform: Platform, game_id: String, value: String) -> Result<Option<String>, String> {
    match platform {
        Platform::Steam => crate::compat_tools::set_steam_compat_tool(&game_id, &value),
        Platform::Lutris => crate::compat_tools::set_lutris_wine_version(&game_id, &value),
        other => Err(format!("Wybór wersji Proton/Wine nie jest obsługiwany dla platformy {}", other.slug())),
    }
}

/// Znaczniki "ostatnio grane" (sekundy Unix) dla WSZYSTKICH gier, jakie
/// lokalny licznik czasu gry kiedykolwiek widział — patrz
/// `playtime::all_entries`. Klucz to `"<platform-slug>:<id>"`, ten sam
/// format co `cover_override_key`/`Settings::game_tags`.
///
/// Celowo zwraca TYLKO tę jedną mapę, nie gotowe "statystyki biblioteki"
/// (najczęściej grane/łączny czas/itp.) — te da się policzyć w całości po
/// stronie frontendu z `gamesStore.games()` (który już ma
/// `playtime_minutes` scalony ze WSZYSTKICH źródeł, patrz
/// `playtime::merge_into`) plus tej jednej dodatkowej mapy, bez
/// duplikowania tej samej logiki agregującej w dwóch miejscach (Rust i
/// TypeScript) dla czegoś, co i tak trzeba pokazać w UI.
#[tauri::command]
pub fn get_playtime_last_played() -> std::collections::HashMap<String, u64> {
    crate::playtime::all_entries()
        .into_iter()
        .filter(|e| e.last_played_at > 0)
        .map(|e| (format!("{}:{}", e.platform_slug, e.game_id), e.last_played_at))
        .collect()
}

// ===================== Kopie zapasowe zapisów =====================

/// Ustawia ręcznie wskazany katalog zapisu dla danej gry — patrz
/// `Settings::game_save_paths`/moduł-dokumentacja `cloud_saves.rs` po
/// wyjaśnienie, dlaczego to ręczne, nie automatycznie wykrywane.
#[tauri::command]
pub fn set_game_save_path(state: State<AppState>, platform: Platform, game_id: String, path: Option<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    let key = cover_override_key(platform, &game_id);
    match path {
        Some(p) if !p.trim().is_empty() => {
            settings.game_save_paths.insert(key, p.trim().to_string());
        }
        _ => {
            settings.game_save_paths.remove(&key);
        }
    }
    settings.save().map_err(|e| e.to_string())
}

fn require_backup_root(state: &State<AppState>) -> Result<std::path::PathBuf, String> {
    let dir = state
        .settings
        .lock()
        .unwrap()
        .cloud_saves_backup_dir
        .clone()
        .ok_or("Nie ustawiono katalogu na kopie zapasowe zapisów (Ustawienia → Kopie zapasowe)")?;
    Ok(std::path::PathBuf::from(dir))
}

fn require_save_path(state: &State<AppState>, platform: Platform, game_id: &str) -> Result<String, String> {
    let key = cover_override_key(platform, game_id);
    state
        .settings
        .lock()
        .unwrap()
        .game_save_paths
        .get(&key)
        .cloned()
        .ok_or_else(|| "Nie ustawiono katalogu zapisu dla tej gry".to_string())
}

#[tauri::command]
pub fn backup_game_save(state: State<AppState>, platform: Platform, game_id: String) -> Result<crate::cloud_saves::BackupEntry, String> {
    let backup_root = require_backup_root(&state)?;
    let save_path = require_save_path(&state, platform, &game_id)?;
    crate::cloud_saves::backup_now(&backup_root, platform.slug(), &game_id, &save_path)
}

#[tauri::command]
pub fn list_game_save_backups(state: State<AppState>, platform: Platform, game_id: String) -> Vec<crate::cloud_saves::BackupEntry> {
    let Ok(backup_root) = require_backup_root(&state) else {
        return Vec::new();
    };
    crate::cloud_saves::list_backups(&backup_root, platform.slug(), &game_id)
}

#[tauri::command]
pub fn restore_game_save_backup(
    state: State<AppState>,
    platform: Platform,
    game_id: String,
    backup_file_name: String,
) -> Result<(), String> {
    let backup_root = require_backup_root(&state)?;
    let save_path = require_save_path(&state, platform, &game_id)?;
    crate::cloud_saves::restore_backup(&backup_root, platform.slug(), &game_id, &backup_file_name, &save_path)
}

// ===================== Kontrolery =====================

#[tauri::command]
pub fn list_connected_controllers() -> Vec<crate::controllers::ConnectedController> {
    crate::controllers::list_connected_controllers()
}

/// Zapisuje (lub czyści, gdy `config` jest `None`/puste) string
/// `SDL_GAMECONTROLLERCONFIG` dla danej gry — patrz moduł-dokumentacja
/// `controllers.rs`. Wstosowany dopiero przy NASTĘPNYM uruchomieniu tej
/// gry (patrz `launcher::launch_game`), nie ma efektu na już działający
/// proces.
#[tauri::command]
pub fn set_game_controller_config(state: State<AppState>, platform: Platform, game_id: String, config: Option<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    let key = cover_override_key(platform, &game_id);
    match config {
        Some(c) if !c.trim().is_empty() => {
            settings.game_controller_configs.insert(key, c.trim().to_string());
        }
        _ => {
            settings.game_controller_configs.remove(&key);
        }
    }
    settings.save().map_err(|e| e.to_string())
}

// ===================== Zaawansowane: uruchamianie i powiadomienia =====================

/// Ustawia globalny prefiks uruchamiania (`Settings::custom_launch_prefix`,
/// patrz `launcher::wrap_with_custom_prefix`) — `None`/pusty string
/// czyści ustawienie (powrót do zwykłego uruchamiania bez dodatkowego
/// opakowania).
#[tauri::command]
pub fn set_custom_launch_prefix(state: State<AppState>, prefix: Option<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.custom_launch_prefix = prefix.filter(|p| !p.trim().is_empty());
    settings.save().map_err(|e| e.to_string())
}

/// Ustawia `Settings::crash_detection_threshold_seconds` (patrz jego
/// dokumentacja w `settings.rs` — jedno, dzielone źródło prawdy zamiast
/// trzech niezależnie wpisanych na sztywno stałych). Ograniczone do
/// rozsądnego zakresu 1-120s: `0` zamieniałoby w "crash" KAŻDE
/// zamknięcie gry (nawet natychmiast po normalnym wyjściu), a wartości
/// rzędu minut przestałyby odróżniać crash od po prostu bardzo krótkiej,
/// ale celowej sesji.
#[tauri::command]
pub fn set_crash_detection_threshold(state: State<AppState>, seconds: u64) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.crash_detection_threshold_seconds = seconds.clamp(1, 120);
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_notification_settings(
    state: State<AppState>,
    notifications: crate::settings::NotificationSettings,
) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.notifications = notifications;
    settings.save().map_err(|e| e.to_string())
}

// ===================== Synchronizacja ustawień (eksport/import) =====================
//
// Hacker Mode nie ma żadnej własnej infrastruktury serwerowej do
// synchronizacji ustawień między urządzeniami — zamiast tego pozwala
// wyeksportować cały `Settings` (WŁĄCZNIE z tagami, ścieżkami zapisów,
// mapowaniami kontrolera itd.) do jednego pliku JSON, który użytkownik
// sam przeniesie na inne urządzenie (przez USB, tę samą chmurę co kopie
// zapasowe zapisów, cokolwiek) i zaimportuje tam. To ta sama filozofia co
// `cloud_saves.rs`: uczciwe przyznanie, że nie ma prawdziwej synchronizacji
// w tle, tylko ręczny mechanizm eksport/import.

/// Zwraca AKTUALNE ustawienia jako sformatowany JSON — dokładnie to, co
/// leży w `config.json`, gotowe do zapisania przez frontend jako plik
/// (przez `tauri-plugin-dialog`, ten sam mechanizm co inne dialogi
/// zapisu pliku w tej aplikacji).
#[tauri::command]
pub fn export_settings(state: State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().unwrap();
    serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())
}

/// Wczytuje ustawienia z podanego JSON-a (treść pliku wybranego przez
/// użytkownika) i NADPISUJE nimi aktualne — CAŁKOWICIE, nie scala pole po
/// polu. Walidacja to po prostu próba deserializacji do `Settings`:
/// nieprawidłowy JSON albo brakujące wymagane pole (żadne dziś nie jest
/// wymagane — wszystkie mają `#[serde(default)]` albo są `Option`, patrz
/// `settings.rs`) zwróci czytelny błąd zamiast po cichu zaakceptować coś
/// zepsutego.
#[tauri::command]
pub fn import_settings(state: State<AppState>, json: String) -> Result<(), String> {
    let parsed: Settings = serde_json::from_str(&json).map_err(|e| format!("Nieprawidłowy plik ustawień: {e}"))?;
    let mut settings = state.settings.lock().unwrap();
    *settings = parsed;
    settings.save().map_err(|e| e.to_string())
}

// ===================== Zewnętrzne źródła (informacyjnie) =====================

/// Gry/programy wykryte poza Hacker Mode — patrz moduł-dokumentacja
/// `external_sources.rs` po pełne wyjaśnienie zakresu (WYŁĄCZNIE
/// informacyjne, nie pełna integracja).
#[tauri::command]
pub fn list_external_apps() -> Vec<crate::external_sources::ExternalApp> {
    crate::external_sources::list_all()
}

// ===================== Historia sesji =====================

#[tauri::command]
pub fn get_game_session_history(platform: Platform, game_id: String) -> Vec<crate::playtime::SessionEntry> {
    crate::playtime::get_session_history(platform.slug(), &game_id)
}
