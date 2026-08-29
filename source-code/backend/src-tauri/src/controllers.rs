use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectedController {
    pub name: String,
    /// Ścieżka handlera zdarzeń (np. `event5`) — czysto informacyjna,
    /// przydatna przy diagnozowaniu, o które urządzenie chodzi, gdy
    /// podłączonych jest kilka podobnych padów.
    pub handler: String,
}

/// Wykrywa podłączone kontrolery przez `/proc/bus/input/devices` —
/// standardowy plik jądra Linuksa listujący WSZYSTKIE urządzenia
/// wejściowe (klawiatury, myszy, pady) w formacie bloków tekstowych
/// rozdzielonych pustą linią, każdy z linią `N: Name="..."` i linią
/// `H: Handlers=...`. Filtrujemy po obecności handlera `js` (joystick) —
/// to właśnie odróżnia pady/joysticki od reszty urządzeń wejściowych w
/// tym pliku, bez potrzeby pełnego parsowania bitmap `EV=`/`B=` (które
/// technicznie dokładniej odróżniłyby urządzenia, ale są dużo bardziej
/// złożone do sparsowania dla korzyści, która tu nie jest potrzebna: chcemy
/// tylko pokazać użytkownikowi listę "to wygląda jak pad", nie
/// klasyfikować urządzeń ze stuprocentową pewnością).
pub fn list_connected_controllers() -> Vec<ConnectedController> {
    let Ok(content) = std::fs::read_to_string(devices_path()) else {
        return Vec::new();
    };
    parse_input_devices(&content)
}

fn devices_path() -> PathBuf {
    PathBuf::from("/proc/bus/input/devices")
}

fn parse_input_devices(content: &str) -> Vec<ConnectedController> {
    let mut out = Vec::new();
    for block in content.split("\n\n") {
        let name = block
            .lines()
            .find_map(|l| l.strip_prefix("N: Name=\""))
            .and_then(|s| s.strip_suffix('"'))
            .map(str::to_string);
        let handlers_line = block.lines().find_map(|l| l.strip_prefix("H: Handlers="));

        let Some(handlers_line) = handlers_line else { continue };
        let Some(js_handler) = handlers_line.split_whitespace().find(|h| h.starts_with("js")) else {
            continue;
        };
        let Some(name) = name else { continue };

        out.push(ConnectedController { name, handler: js_handler.to_string() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kształt oparty na realnym formacie `/proc/bus/input/devices`
    /// (standardowy, udokumentowany format jądra Linuksa, `Documentation/
    /// input/input.rst`) — dwa urządzenia: klawiatura (bez handlera `js`,
    /// więc pomijana) i pad Xbox (z handlerem `js0`).
    const SAMPLE_DEVICES: &str = "I: Bus=0003 Vendor=046d Product=c31c Version=0111\nN: Name=\"Logitech USB Keyboard\"\nP: Phys=usb-0000:00:14.0-1/input0\nS: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-1/1-1:1.0/0003:046D:C31C.0001/input/input5\nU: Uniq=\nH: Handlers=sysrq kbd leds event5\nB: PROP=0\nB: EV=120013\n\nI: Bus=0003 Vendor=045e Product=028e Version=0110\nN: Name=\"Microsoft X-Box 360 pad\"\nP: Phys=usb-0000:00:14.0-2/input0\nS: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/0003:045E:028E.0002/input/input6\nU: Uniq=\nH: Handlers=js0 event6\nB: PROP=0\nB: EV=20000b\n";

    #[test]
    fn parses_only_devices_with_joystick_handler() {
        let controllers = parse_input_devices(SAMPLE_DEVICES);
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].name, "Microsoft X-Box 360 pad");
        assert_eq!(controllers[0].handler, "js0");
    }

    #[test]
    fn empty_input_returns_empty_list() {
        assert!(parse_input_devices("").is_empty());
    }

    #[test]
    fn block_without_name_is_skipped_without_panicking() {
        let content = "H: Handlers=js0 event6\n";
        assert!(parse_input_devices(content).is_empty());
    }
}
