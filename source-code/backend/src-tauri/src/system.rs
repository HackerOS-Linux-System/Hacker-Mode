use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct ActionResult {
    pub ok: bool,
    pub message: String,
}

fn run(program: &str, args: &[&str]) -> ActionResult {
    match Command::new(program).args(args).output() {
        Ok(out) if out.status.success() => ActionResult {
            ok: true,
            message: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        },
        Ok(out) => ActionResult {
            ok: false,
            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        },
        Err(e) => ActionResult {
            ok: false,
            message: format!("nie znaleziono `{program}`: {e}"),
        },
    }
}

// --- Audio (PipeWire przez wpctl, z fallbackiem na pactl/PulseAudio) ----

pub fn audio_volume_up() -> ActionResult {
    if which::which("wpctl").is_ok() {
        run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"])
    } else {
        run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", "+5%"])
    }
}

pub fn audio_volume_down() -> ActionResult {
    if which::which("wpctl").is_ok() {
        run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"])
    } else {
        run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", "-5%"])
    }
}

pub fn audio_toggle_mute() -> ActionResult {
    if which::which("wpctl").is_ok() {
        run("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
    } else {
        run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
    }
}

// --- Wyświetlacz ----------------------------------------------------------

pub fn display_brightness_up() -> ActionResult {
    run("brightnessctl", &["set", "+5%"])
}

pub fn display_brightness_down() -> ActionResult {
    run("brightnessctl", &["set", "5%-"])
}

// --- Sieć ------------------------------------------------------------------

pub fn network_toggle_wifi() -> ActionResult {
    // Odczytaj aktualny stan i przełącz.
    let status = run("nmcli", &["radio", "wifi"]);
    let enabling = status.message.trim() != "enabled";
    run(
        "nmcli",
        &["radio", "wifi", if enabling { "on" } else { "off" }],
    )
}

pub fn network_list_wifi() -> ActionResult {
    run(
        "nmcli",
        &["-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
    )
}

pub fn network_connect_wifi(ssid: &str, password: Option<&str>) -> ActionResult {
    match password {
        Some(pass) => run("nmcli", &["dev", "wifi", "connect", ssid, "password", pass]),
        None => run("nmcli", &["dev", "wifi", "connect", ssid]),
    }
}

// --- Bluetooth ---------------------------------------------------------

pub fn bluetooth_scan() -> ActionResult {
    run("bluetoothctl", &["--timeout", "5", "scan", "on"])
}

pub fn bluetooth_pair(mac_address: &str) -> ActionResult {
    run("bluetoothctl", &["pair", mac_address])
}

// --- Zasilanie ---------------------------------------------------------

pub fn power_set_profile(profile: &str) -> ActionResult {
    let normalized = match profile {
        "power_saving" => "power-saver",
        "performance" => "performance",
        _ => "balanced",
    };
    run("powerprofilesctl", &["set", normalized])
}

pub fn power_shutdown() -> ActionResult {
    run("systemctl", &["poweroff"])
}

pub fn power_restart() -> ActionResult {
    run("systemctl", &["reboot"])
}

pub fn power_sleep() -> ActionResult {
    run("systemctl", &["suspend"])
}
