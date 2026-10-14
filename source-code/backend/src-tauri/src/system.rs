use serde::Serialize;
use std::process::{Command, Stdio};

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

/// SSID Wi-Fi: 1-32 bajty, bez znaków sterujących i nie zaczynający się od
/// `-` (żeby `nmcli` nie potraktował go jako opcji).
pub fn validate_ssid(ssid: &str) -> Result<(), String> {
    if ssid.is_empty() || ssid.len() > 32 {
        return Err("SSID musi mieć od 1 do 32 bajtów".into());
    }
    if ssid.starts_with('-') {
        return Err("SSID nie może zaczynać się od „-”".into());
    }
    if ssid.chars().any(char::is_control) {
        return Err("SSID zawiera niedozwolone znaki sterujące".into());
    }
    Ok(())
}

/// Hasło WPA/WPA2/WPA3-Personal: 8-63 znaki ASCII (drukowalne) albo 64 znaki
/// hex (klucz surowy). Odrzucamy znaki sterujące - zwłaszcza nowe linie,
/// bo hasło jest wysyłane do `nmcli --ask` linią na stdin.
pub fn validate_wifi_password(password: &str) -> Result<(), String> {
    if password.chars().any(char::is_control) {
        return Err("Hasło zawiera niedozwolone znaki sterujące".into());
    }
    let len = password.len();
    let is_hex_key = len == 64 && password.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex_key || (8..=63).contains(&len) {
        Ok(())
    } else {
        Err("Hasło WPA musi mieć 8-63 znaków (albo 64 znaki szesnastkowe)".into())
    }
}

/// Adres MAC urządzenia Bluetooth w postaci `AA:BB:CC:DD:EE:FF`.
pub fn validate_mac(mac: &str) -> Result<(), String> {
    let parts: Vec<&str> = mac.split(':').collect();
    let ok = parts.len() == 6 && parts.iter().all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()));
    if ok {
        Ok(())
    } else {
        Err("Niepoprawny adres MAC (oczekiwano AA:BB:CC:DD:EE:FF)".into())
    }
}

pub fn network_connect_wifi(ssid: &str, password: Option<&str>) -> ActionResult {
    if let Err(message) = validate_ssid(ssid) {
        return ActionResult { ok: false, message };
    }
    if let Some(pass) = password.filter(|p| !p.is_empty()) {
        if let Err(message) = validate_wifi_password(pass) {
            return ActionResult { ok: false, message };
        }
    }
    let password = password.filter(|p| !p.is_empty());
    match password {
        Some(pass) => connect_wifi_with_password(ssid, pass),
        None => run("nmcli", &["dev", "wifi", "connect", ssid]),
    }
}

/// BUGFIX: poprzednia wersja przekazywała hasło Wi-Fi wprost jako argument
/// CLI (`nmcli dev wifi connect <ssid> password <hasło>`) — przez cały
/// czas trwania procesu `nmcli` taki argument jest widoczny w plaintext
/// dla KAŻDEGO innego procesu w systemie przez `ps aux` czy
/// `/proc/<pid>/cmdline` (to nie jest teoretyczne ryzyko — `ps aux` bez
/// żadnych uprawnień pokazuje pełną listę argumentów wszystkich procesów
/// wszystkich użytkowników na typowej instalacji Linuksa). Na
/// jednoużytkownikowej maszynie do grania ryzyko jest niskie, ale to
/// tania do naprawienia higiena.
///
/// Zamiast tego używamy `nmcli --ask`, które dla brakującego sekretu samo
/// dopytuje o hasło (standardowo — interaktywnie z terminala, ale radzi
/// sobie też z odczytem z podanego na sztywno strumienia `stdin`, co jest
/// udokumentowanym/powszechnie stosowanym sposobem automatyzacji `nmcli`
/// w skryptach) — więc podajemy hasło przez potok do `stdin` procesu
/// potomnego zamiast jako argument.
///
/// Uwaga: to środowisko deweloperskie nie ma uruchomionego NetworkManagera
/// (nie da się tu tego przetestować end-to-end na żywo) — jeśli
/// `--ask` z jakiegoś powodu zachowa się inaczej niż udokumentowano na
/// konkretnej wersji `nmcli`/dystrybucji, daj znać, to dobierzemy inny
/// mechanizm (np. tymczasowy plik profilu połączenia z
/// `chmod 600`, kasowany od razu po użyciu).
fn connect_wifi_with_password(ssid: &str, password: &str) -> ActionResult {
    use std::io::Write;

    let mut child = match Command::new("nmcli")
        .args(["--ask", "dev", "wifi", "connect", ssid])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return ActionResult { ok: false, message: format!("nie znaleziono `nmcli`: {e}") };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        // `--ask` pyta o SSID i hasło po kolei, jeśli nie zostały podane w
        // argumentach — SSID już podaliśmy jako argument, więc na stdin
        // czeka tylko hasło.
        let _ = writeln!(stdin, "{password}");
    }

    // `nmcli --ask` potrafi czekać w nieskończoność, jeśli oczekuje na
    // coś, czego nie dostanie (np. inna wersja nmcli pyta o więcej niż
    // hasło). Zamiast wieszać wątek komendy Tauri, czekamy maksymalnie
    // `NMCLI_TIMEOUT` i zabijamy proces.
    const NMCLI_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
    let deadline = std::time::Instant::now() + NMCLI_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return ActionResult {
                    ok: false,
                    message: "`nmcli` nie odpowiedziało w ciągu 45 s - przerwano połączenie".into(),
                };
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => {
                return ActionResult { ok: false, message: format!("błąd podczas oczekiwania na `nmcli`: {e}") };
            }
        }
    }

    match child.wait_with_output() {
        Ok(out) if out.status.success() => {
            ActionResult { ok: true, message: String::from_utf8_lossy(&out.stdout).trim().to_string() }
        }
        Ok(out) => ActionResult { ok: false, message: String::from_utf8_lossy(&out.stderr).trim().to_string() },
        Err(e) => ActionResult { ok: false, message: format!("błąd podczas oczekiwania na `nmcli`: {e}") },
    }
}

// --- Bluetooth ---------------------------------------------------------

pub fn bluetooth_scan() -> ActionResult {
    run("bluetoothctl", &["--timeout", "5", "scan", "on"])
}

pub fn bluetooth_pair(mac_address: &str) -> ActionResult {
    if let Err(message) = validate_mac(mac_address) {
        return ActionResult { ok: false, message };
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssid_validation() {
        assert!(validate_ssid("Dom_5G").is_ok());
        assert!(validate_ssid("").is_err());
        assert!(validate_ssid(&"a".repeat(33)).is_err());
        assert!(validate_ssid("-oops").is_err());
        assert!(validate_ssid("zła\nnazwa").is_err());
    }

    #[test]
    fn wifi_password_validation() {
        assert!(validate_wifi_password("12345678").is_ok());
        assert!(validate_wifi_password(&"a".repeat(63)).is_ok());
        assert!(validate_wifi_password(&"A".repeat(64)).is_ok()); // 64 znaki hex
        assert!(validate_wifi_password(&"g".repeat(64)).is_err()); // 64 znaki, ale nie hex
        assert!(validate_wifi_password("krotkie").is_err());
        assert!(validate_wifi_password("dobre-haslo\nmalicious").is_err());
    }

    #[test]
    fn mac_validation() {
        assert!(validate_mac("AA:bb:0C:12:34:56").is_ok());
        assert!(validate_mac("AA:bb:0C:12:34").is_err());
        assert!(validate_mac("AA-bb-0C-12-34-56").is_err());
        assert!(validate_mac("--help").is_err());
        assert!(validate_mac("ZZ:bb:0C:12:34:56").is_err());
    }

    #[test]
    fn invalid_input_never_reaches_nmcli() {
        let r = network_connect_wifi("-x", None);
        assert!(!r.ok);
        assert!(r.message.contains("SSID"));
        let r = network_connect_wifi("Dom", Some("krotkie"));
        assert!(!r.ok);
        assert!(r.message.contains("Hasło"));
    }
}
