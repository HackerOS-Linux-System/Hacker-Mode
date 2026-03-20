// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

#[derive(Clone, serde::Serialize)]
struct WifiNetwork {
    ssid: String,
    signal: u8,
    security: bool,
    connected: bool,
}

#[tauri::command]
fn get_user_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home".into())
}

#[tauri::command]
fn set_volume(volume: u32) -> Result<(), String> {
    let vol = volume.to_string();
    let output = Command::new("pactl")
    .args(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", vol)])
    .output()
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn toggle_mute() -> Result<(), String> {
    let output = Command::new("pactl")
    .args(&["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
    .output()
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn set_brightness(brightness: u32) -> Result<(), String> {
    let bri = brightness.to_string();
    let output = Command::new("brightnessctl")
    .args(&["set", &format!("{}%", bri)])
    .output()
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn toggle_wifi(enabled: bool) -> Result<(), String> {
    let state = if enabled { "on" } else { "off" };
    let output = Command::new("nmcli")
    .args(&["radio", "wifi", state])
    .output()
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn scan_wifi() -> Result<Vec<WifiNetwork>, String> {
    let output = Command::new("nmcli")
    .args(&["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"])
    .output()
    .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let ssid = parts[0].to_string();
            let signal = parts[1].parse::<u8>().unwrap_or(0);
            let security = !parts[2].is_empty() && parts[2] != "--";
            networks.push(WifiNetwork {
                ssid,
                signal,
                security,
                connected: false,
            });
        }
    }
    Ok(networks)
}

#[tauri::command]
fn connect_wifi(ssid: String, password: Option<String>) -> Result<bool, String> {
    let mut cmd = Command::new("nmcli");
    cmd.args(&["dev", "wifi", "connect", &ssid]);
    if let Some(pwd) = password {
        cmd.args(&["password", &pwd]);
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    Ok(output.status.success())
}

#[tauri::command]
fn set_power_profile(profile: String) -> Result<(), String> {
    let output = Command::new("powerprofilesctl")
    .args(&["set", &profile])
    .output()
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn launch_app(command: String) -> Result<(), String> {
    Command::new("sh")
    .arg("-c")
    .arg(format!("nohup {} >/dev/null 2>&1 &", command))
    .spawn()
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn system_action(action: String) -> Result<(), String> {
    let cmd = match action.as_str() {
        "shutdown" => "systemctl poweroff",
        "restart" => "systemctl reboot",
        "sleep" => "systemctl suspend",
        "switch_plasma" => "qdbus org.kde.ksmserver /KSMServer logout 0 0 0",
        _ => return Err(format!("Unknown action: {}", action)),
    };
    Command::new("sh")
    .arg("-c")
    .arg(cmd)
    .spawn()
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        get_user_home,
        set_volume,
        toggle_mute,
        set_brightness,
        toggle_wifi,
        scan_wifi,
        connect_wifi,
        set_power_profile,
        launch_app,
        system_action
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
