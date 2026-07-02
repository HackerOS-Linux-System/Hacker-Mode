use crate::platforms::{self, native::ManualEntry, Game};
use crate::system::{audio, bluetooth, brightness, locale, power, timezone, wifi};
use tauri::State;

#[derive(Default)]
pub struct AppState; // miejsce na cache listy gier, uchwyt sesji itd.

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        // --- biblioteka gier ---
        list_all_games,
        launch_game,
        add_manual_game,
        // --- Hacker Menu (odpowiednik Steam Menu) ---
        open_hacker_menu,
        close_hacker_menu,
        quit_to_desktop,
        suspend_system,
        shutdown_system,
        // --- WiFi ---
        wifi_is_enabled,
        wifi_set_enabled,
        wifi_scan,
        wifi_connect,
        wifi_forget,
        // --- Bluetooth ---
        bt_is_enabled,
        bt_set_enabled,
        bt_scan,
        bt_connect,
        bt_forget,
        // --- Audio ---
        audio_get_volume,
        audio_set_volume,
        audio_get_muted,
        audio_set_muted,
        // --- Jasność ---
        brightness_get,
        brightness_set,
        // --- Zasilanie / bateria ---
        power_get_profile,
        power_set_profile,
        power_get_battery,
        // --- Strefa czasowa ---
        tz_get,
        tz_list,
        tz_set,
        // --- Język ---
        locale_get,
        locale_list,
        locale_set,
    ]
}

// ---------- Biblioteka gier ----------

#[tauri::command]
async fn list_all_games() -> Result<Vec<Game>, String> {
    platforms::list_all_games().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn launch_game(game: Game) -> Result<(), String> {
    platforms::launch(&game).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_manual_game(entry: ManualEntry) -> Result<(), String> {
    platforms::native::add_manual_entry(entry)
        .await
        .map_err(|e| e.to_string())
}

// ---------- Hacker Menu ----------
// Odpowiednik przycisku "Steam" w Steam GamepadUI: overlay z zasilaniem,
// ustawieniami, powrotem do biblioteki. Emitujemy zdarzenie do kompozytora
// (w trybie "default") żeby pokazał UI jako nakładkę nad grą; w trybie "ui"
// to zwyczajnie zmiana widoku w SPA.

#[tauri::command]
async fn open_hacker_menu(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("hacker-menu:open", ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn close_hacker_menu(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("hacker-menu:close", ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn quit_to_desktop() -> Result<(), String> {
    // W trybie "default" (własny kompozytor z poziomu SDDM) to powinno
    // zakończyć sesję i wrócić do ekranu logowania SDDM.
    std::process::exit(0);
}

#[tauri::command]
async fn suspend_system() -> Result<(), String> {
    tokio::process::Command::new("systemctl")
        .arg("suspend")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn shutdown_system() -> Result<(), String> {
    tokio::process::Command::new("systemctl")
        .arg("poweroff")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- WiFi ----------

#[tauri::command]
async fn wifi_is_enabled() -> Result<bool, String> {
    wifi::is_enabled().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn wifi_set_enabled(enabled: bool) -> Result<(), String> {
    wifi::set_enabled(enabled).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn wifi_scan() -> Result<Vec<wifi::WifiNetwork>, String> {
    wifi::scan_networks().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn wifi_connect(ssid: String, password: Option<String>) -> Result<(), String> {
    wifi::connect(&ssid, password.as_deref())
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn wifi_forget(ssid: String) -> Result<(), String> {
    wifi::forget(&ssid).await.map_err(|e| e.to_string())
}

// ---------- Bluetooth ----------

#[tauri::command]
async fn bt_is_enabled() -> Result<bool, String> {
    bluetooth::is_enabled().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn bt_set_enabled(enabled: bool) -> Result<(), String> {
    bluetooth::set_enabled(enabled).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn bt_scan() -> Result<Vec<bluetooth::BtDevice>, String> {
    bluetooth::scan_devices().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn bt_connect(address: String) -> Result<(), String> {
    bluetooth::pair_and_connect(&address)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn bt_forget(address: String) -> Result<(), String> {
    bluetooth::forget(&address).await.map_err(|e| e.to_string())
}

// ---------- Audio ----------

#[tauri::command]
async fn audio_get_volume() -> Result<u8, String> {
    audio::get_volume().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn audio_set_volume(percent: u8) -> Result<(), String> {
    audio::set_volume(percent).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn audio_get_muted() -> Result<bool, String> {
    audio::is_muted().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn audio_set_muted(muted: bool) -> Result<(), String> {
    audio::set_muted(muted).await.map_err(|e| e.to_string())
}

// ---------- Jasność ----------

#[tauri::command]
async fn brightness_get() -> Result<u8, String> {
    brightness::get_percent().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn brightness_set(percent: u8) -> Result<(), String> {
    brightness::set_percent(percent).await.map_err(|e| e.to_string())
}

// ---------- Zasilanie ----------

#[tauri::command]
async fn power_get_profile() -> Result<power::PowerProfile, String> {
    power::get_profile().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn power_set_profile(profile: power::PowerProfile) -> Result<(), String> {
    power::set_profile(profile).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn power_get_battery() -> Result<Option<power::BatteryInfo>, String> {
    power::get_battery_info().await.map_err(|e| e.to_string())
}

// ---------- Strefa czasowa ----------

#[tauri::command]
async fn tz_get() -> Result<String, String> {
    timezone::get_current().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn tz_list() -> Result<Vec<String>, String> {
    timezone::list_available().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn tz_set(tz: String) -> Result<(), String> {
    timezone::set_timezone(&tz).await.map_err(|e| e.to_string())
}

// ---------- Język ----------

#[tauri::command]
async fn locale_get() -> Result<String, String> {
    locale::get_current_locale().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn locale_list() -> Result<Vec<String>, String> {
    locale::list_available_locales().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn locale_set(locale: String) -> Result<(), String> {
    locale::set_locale(&locale).await.map_err(|e| e.to_string())
}
