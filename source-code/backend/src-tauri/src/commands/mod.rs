pub mod stores;

use tauri::{AppHandle, State};

use crate::settings::Settings;
use crate::state::AppState;
use crate::system::{self, ActionResult};
use stores::{Game, Platform};

// --- Biblioteka gier -----------------------------------------------------

#[tauri::command]
pub fn list_games(state: State<AppState>) -> Vec<Game> {
    let settings = state.settings.lock().unwrap();
    let credentials = match (&settings.steam_api_key, &settings.steam_id64) {
        (Some(key), Some(id)) if !key.is_empty() && !id.is_empty() => Some((key.clone(), id.clone())),
        _ => None,
    };
    drop(settings);
    stores::list_all_games(credentials.as_ref().map(|(k, i)| (k.as_str(), i.as_str())))
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
