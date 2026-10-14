use serde::Serialize;

use crate::settings::Compositor;
use crate::util::command_exists;

#[derive(Debug, Clone, Serialize)]
pub struct ToolCheck {
    pub id: &'static str,
    pub purpose: &'static str,
    pub found: bool,
    /// Brak tego narzędzia uniemożliwia działanie sesji / głównej funkcji.
    pub critical: bool,
    pub hint: &'static str,
}

fn tool(id: &'static str, purpose: &'static str, critical: bool, hint: &'static str) -> ToolCheck {
    ToolCheck { id, purpose, found: command_exists(id), critical, hint }
}

pub fn check_dependencies() -> Vec<ToolCheck> {
    let compositor = crate::compositor::active();
    vec![
        tool("labwc", "Kompozytor sesji (domyślny)", compositor == Compositor::Labwc,
             "Zainstaluj pakiet `labwc` (Arch: pacman -S labwc, Debian/Ubuntu: apt install labwc)."),
        tool("comphwde", "Kompozytor HWDE (opcjonalny)", compositor == Compositor::Comphwde,
             "Zainstaluj comphwde z projektu HWDE albo ustaw \"compositor\": \"labwc\" w config.json."),
        tool("Xwayland", "Gry X11/Wine pod Wayland", false, "Zainstaluj `xorg-xwayland` / `xwayland`."),
        tool("steam", "Biblioteka Steam", false, "Zainstaluj Steam (pakiet lub Flatpak)."),
        tool("lutris", "Biblioteka Lutris", false, "Zainstaluj `lutris`."),
        tool("sqlite3", "Odczyt/zmiany bazy Lutrisa (czas gry, odinstalowanie)", false, "Zainstaluj `sqlite3`."),
        tool("legendary", "Epic Games", false, "Zainstaluj z poziomu pierwszego uruchomienia (venv) lub `pip install legendary-gl`."),
        tool("gogdl", "GOG", false, "Zainstaluj z poziomu pierwszego uruchomienia (venv)."),
        tool("nile", "Amazon Games", false, "Zainstaluj z poziomu pierwszego uruchomienia (venv)."),
        tool("nmcli", "Wi-Fi (NetworkManager)", false, "Zainstaluj `networkmanager`."),
        tool("bluetoothctl", "Bluetooth (BlueZ)", false, "Zainstaluj `bluez-utils`."),
        tool("wpctl", "Głośność (PipeWire; fallback: pactl)", false, "Zainstaluj `wireplumber` lub `pulseaudio-utils`."),
        tool("brightnessctl", "Jasność ekranu", false, "Zainstaluj `brightnessctl`."),
        tool("powerprofilesctl", "Profile zasilania", false, "Zainstaluj `power-profiles-daemon`."),
        tool("pgrep", "Wykrywanie działających klientów", true, "Zainstaluj `procps`."),
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct FormatCheck {
    pub name: &'static str,
    /// `Some(true)` = zgodne, `Some(false)` = PROBLEM, `None` = brak danych
    /// do sprawdzenia (np. Steam nie jest zainstalowany).
    pub ok: Option<bool>,
    pub detail: String,
}

fn check(name: &'static str, ok: Option<bool>, detail: impl Into<String>) -> FormatCheck {
    FormatCheck { name, ok, detail: detail.into() }
}

/// Tylko odczyt: uruchamia parsery na prawdziwych plikach.
pub fn verify_external_formats() -> Vec<FormatCheck> {
    let mut out = Vec::new();

    // Steam: config.vdf
    match crate::compat_tools::config_vdf_path() {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(content) => {
                let structural = crate::compat_tools::vdf_is_structurally_valid(&content);
                let has_section = content.contains("CompatToolMapping");
                out.push(check(
                    "Steam config.vdf",
                    Some(structural),
                    format!("{} - struktura {}, sekcja CompatToolMapping {}", path.display(),
                        if structural { "poprawna" } else { "USZKODZONA" },
                        if has_section { "jest" } else { "nie istnieje jeszcze (zapis ustawień narzędzia zgodności się nie powiedzie, dopóki Steam jej nie utworzy)" }),
                ));
            }
            Err(e) => out.push(check("Steam config.vdf", Some(false), format!("{}: {e}", path.display()))),
        },
        None => out.push(check("Steam config.vdf", None, "nie znaleziono instalacji Steam")),
    }

    // Steam: registry.vdf (lokalne śledzenie działającej gry)
    match crate::commands::stores::steam::local_running_appid() {
        Some(running) => out.push(check("Steam registry.vdf (RunningAppID)", Some(true), match running {
            Some(id) => format!("odczyt działa; aktualnie działa AppID {id}"),
            None => "odczyt działa; żadna gra nie jest uruchomiona".to_string(),
        })),
        None => out.push(check("Steam registry.vdf (RunningAppID)", None,
            "brak pliku - Steam jeszcze nie był uruchomiony; śledzenie użyje Web API (jeśli skonfigurowane)")),
    }

    // Lutris
    let pga = crate::commands::stores::lutris::pga_db_path();
    if pga.is_file() {
        if command_exists("sqlite3") {
            let res = std::process::Command::new("sqlite3").arg(&pga).arg("SELECT count(*) FROM games;").output();
            match res {
                Ok(o) if o.status.success() => out.push(check("Lutris pga.db", Some(true),
                    format!("tabela games odczytana, wpisów: {}", String::from_utf8_lossy(&o.stdout).trim()))),
                Ok(o) => out.push(check("Lutris pga.db", Some(false), String::from_utf8_lossy(&o.stderr).trim().to_string())),
                Err(e) => out.push(check("Lutris pga.db", Some(false), e.to_string())),
            }
        } else {
            out.push(check("Lutris pga.db", Some(false), "baza istnieje, ale brak `sqlite3` w PATH"));
        }
    } else {
        out.push(check("Lutris pga.db", None, "nie znaleziono bazy Lutrisa"));
    }

    // Heroic / Bottles
    let heroic = crate::external_sources::list_heroic_sideloaded();
    out.push(check("Heroic sideload_apps/library.json", if heroic.is_empty() { None } else { Some(true) },
        format!("rozpoznane aplikacje: {}", heroic.len())));
    let bottles = crate::external_sources::list_bottles();
    out.push(check("Bottles bottle.yml", if bottles.is_empty() { None } else { Some(true) },
        format!("rozpoznane butelki: {}", bottles.len())));

    out
}

/// `hacker-mode doctor` - drukuje raport; zwraca kod wyjścia procesu.
pub fn run_cli() -> i32 {
    println!("Hacker Mode {} - diagnostyka", env!("CARGO_PKG_VERSION"));
    println!("Kompozytor: {} (config: {})", crate::compositor::active().as_str(),
        crate::settings::config_path().display());
    println!("\nZależności:");
    let mut failed = false;
    for t in check_dependencies() {
        let mark = if t.found { "OK  " } else if t.critical { "BRAK" } else { "brak" };
        println!("  [{mark}] {:<16} {}", t.id, t.purpose);
        if !t.found && t.critical {
            failed = true;
            println!("         -> {}", t.hint);
        }
    }
    println!("\nFormaty plików zewnętrznych (tylko odczyt):");
    for f in verify_external_formats() {
        let mark = match f.ok { Some(true) => "OK  ", Some(false) => { failed = true; "BŁĄD" }, None => "n/d " };
        println!("  [{mark}] {:<36} {}", f.name, f.detail);
    }
    if failed { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_list_has_unique_ids_and_marks_only_active_compositor_critical() {
        let deps = check_dependencies();
        let mut ids: Vec<_> = deps.iter().map(|d| d.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), deps.len());
        let critical_compositors = deps.iter().filter(|d| (d.id == "labwc" || d.id == "comphwde") && d.critical).count();
        assert_eq!(critical_compositors, 1);
    }

    #[test]
    fn format_verification_never_panics_on_machines_without_launchers() {
        let _ = verify_external_formats();
    }
}
