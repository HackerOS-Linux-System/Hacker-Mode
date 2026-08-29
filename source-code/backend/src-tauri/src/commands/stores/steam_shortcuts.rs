use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutEntry {
    /// 32-bitowy appid TAK JAK zapisany przez Steam w pliku — do
    /// zbudowania działającego `steam://rungameid/...` trzeba go jeszcze
    /// przekształcić w 64-bitową formę, patrz [`rungameid_64`].
    pub appid: u32,
    pub app_name: String,
    /// Ścieżka do pliku wykonywalnego (bez otaczających cudzysłowów, które
    /// Steam zapisuje w samej wartości — patrz `parse_shortcuts_vdf`).
    pub exe: Option<String>,
}

/// Przekształca 32-bitowy `appid` skrótu w 64-bitowy identyfikator gry,
/// jakiego oczekuje `steam://rungameid/<id>` dla skrótów non-Steam —
/// potwierdzone na Valve Developer Wiki.
pub fn rungameid_64(appid: u32) -> u64 {
    ((appid as u64) << 32) | 0x02000000
}

/// Znajduje wszystkie pliki `shortcuts.vdf` pod danym `root` Steam —
/// jeden per profil w `userdata/` (zwykle jest tylko jeden, ale maszyna
/// mogła w przeszłości mieć zalogowane więcej kont).
pub fn find_all_shortcuts(root: &Path) -> Vec<ShortcutEntry> {
    let userdata = root.join("userdata");
    let Ok(entries) = std::fs::read_dir(&userdata) else {
        return Vec::new();
    };

    let mut all = Vec::new();
    for entry in entries.flatten() {
        let shortcuts_path = entry.path().join("config/shortcuts.vdf");
        let Ok(bytes) = std::fs::read(&shortcuts_path) else {
            continue;
        };
        all.extend(parse_shortcuts_vdf(&bytes));
    }
    all
}

/// Wartość w drzewie binarnego VDF — typ zależny od bajtu-znacznika
/// poprzedzającego każdą parę klucz-wartość (patrz moduł-dokumentacja).
#[derive(Debug, Clone)]
enum BinVdfValue {
    Map(Vec<(String, BinVdfValue)>),
    Str(String),
    Int(i32),
}

fn read_cstring(bytes: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < bytes.len() && bytes[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&bytes[start..*pos]).into_owned();
    *pos += 1; // pomiń terminator \0
    Some(s)
}

/// Czyta jedną "mapę" binarnego VDF od bieżącej pozycji, aż do
/// napotkania bajtu `0x08` (koniec tej mapy) albo końca bufora.
/// Rekurencyjnie wywołuje samą siebie dla zagnieżdżonych map (`0x00`) —
/// dzięki temu NIE trzeba nigdzie zakładać z góry dokładnej głębokości
/// zagnieżdżenia ani dokładnej liczby końcowych bajtów `0x08` (różne
/// źródła podają tu niespójne liczby — 2 vs 4 na sam koniec pliku; ten
/// parser jest na to odporny, bo po prostu kończy, gdy zabraknie bajtów
/// albo zamykających znaczników, zamiast liczyć je na sztywno).
fn read_map(bytes: &[u8], pos: &mut usize) -> Option<Vec<(String, BinVdfValue)>> {
    let mut entries = Vec::new();
    loop {
        if *pos >= bytes.len() {
            return Some(entries);
        }
        let tag = bytes[*pos];
        *pos += 1;
        match tag {
            0x08 => return Some(entries),
            0x00 => {
                let key = read_cstring(bytes, pos)?;
                let nested = read_map(bytes, pos)?;
                entries.push((key, BinVdfValue::Map(nested)));
            }
            0x01 => {
                let key = read_cstring(bytes, pos)?;
                let value = read_cstring(bytes, pos)?;
                entries.push((key, BinVdfValue::Str(value)));
            }
            0x02 => {
                let key = read_cstring(bytes, pos)?;
                if *pos + 4 > bytes.len() {
                    return Some(entries);
                }
                let value = i32::from_le_bytes(bytes[*pos..*pos + 4].try_into().ok()?);
                *pos += 4;
                entries.push((key, BinVdfValue::Int(value)));
            }
            _ => {
                // Nieznany/nieoczekiwany bajt (np. inny typ pola, którego
                // nie obsługujemy — Steam ma ich więcej niż te trzy, np.
                // 0x03/0x04 gdzie indziej w rodzinie formatów VDF) —
                // przerywamy CAŁY parsing tego pliku bezpiecznie zamiast
                // ryzykować błędną interpretację reszty bajtów jako
                // czegoś innego niż jest. Lepiej pominąć plik niż
                // wyprodukować śmieciowe dane.
                return None;
            }
        }
    }
}

pub fn parse_shortcuts_vdf(bytes: &[u8]) -> Vec<ShortcutEntry> {
    let mut pos = 0usize;
    let Some(root) = read_map(bytes, &mut pos) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (key, value) in &root {
        if key != "shortcuts" {
            continue;
        }
        let BinVdfValue::Map(entries) = value else { continue };
        for (_, entry_value) in entries {
            let BinVdfValue::Map(fields) = entry_value else { continue };

            let mut appid = None;
            let mut app_name = None;
            let mut exe = None;
            for (fkey, fval) in fields {
                match (fkey.as_str(), fval) {
                    ("appid", BinVdfValue::Int(v)) => appid = Some(*v as u32),
                    ("AppName", BinVdfValue::Str(s)) => app_name = Some(s.clone()),
                    ("Exe", BinVdfValue::Str(s)) => exe = Some(s.trim_matches('"').to_string()),
                    _ => {}
                }
            }

            if let (Some(appid), Some(app_name)) = (appid, app_name) {
                out.push(ShortcutEntry { appid, app_name, exe });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ręcznie zbudowany minimalny plik `shortcuts.vdf` z jednym wpisem —
    /// bajt po bajcie zgodnie ze strukturą z moduł-dokumentacji, żeby
    /// zweryfikować parser bez zależności od prawdziwego pliku Steam
    /// (do którego ten sandbox i tak nie ma dostępu).
    fn build_fixture() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(0x00);
        buf.extend(b"shortcuts\0");
        buf.push(0x00);
        buf.extend(b"0\0");
        buf.push(0x02);
        buf.extend(b"appid\0");
        buf.extend(2882400001u32.to_le_bytes());
        buf.push(0x01);
        buf.extend(b"AppName\0");
        buf.extend(b"Xenonauts\0");
        buf.push(0x01);
        buf.extend(b"Exe\0");
        buf.extend(b"\"/home/user/Games/Xenonauts/Xenonauts.exe\"\0");
        buf.push(0x08); // koniec wpisu "0"
        buf.push(0x08); // koniec mapy "shortcuts"
        buf.push(0x08); // koniec mapy root
        buf
    }

    #[test]
    fn parses_single_shortcut() {
        let fixture = build_fixture();
        let entries = parse_shortcuts_vdf(&fixture);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].app_name, "Xenonauts");
        assert_eq!(entries[0].appid, 2882400001);
        assert_eq!(entries[0].exe.as_deref(), Some("/home/user/Games/Xenonauts/Xenonauts.exe"));
    }

    #[test]
    fn parses_multiple_shortcuts() {
        let mut buf = Vec::new();
        buf.push(0x00);
        buf.extend(b"shortcuts\0");
        for (idx, (appid, name)) in [(111u32, "Xenonauts"), (222u32, "Carrion")].iter().enumerate() {
            buf.push(0x00);
            buf.extend(idx.to_string().as_bytes());
            buf.push(0x00);
            buf.push(0x02);
            buf.extend(b"appid\0");
            buf.extend(appid.to_le_bytes());
            buf.push(0x01);
            buf.extend(b"AppName\0");
            buf.extend(name.as_bytes());
            buf.push(0x00);
            buf.push(0x08);
        }
        buf.push(0x08);
        buf.push(0x08);

        let entries = parse_shortcuts_vdf(&buf);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].app_name, "Xenonauts");
        assert_eq!(entries[1].app_name, "Carrion");
    }

    #[test]
    fn empty_bytes_returns_empty() {
        assert!(parse_shortcuts_vdf(&[]).is_empty());
    }

    #[test]
    fn rungameid_64_matches_documented_formula() {
        // Dolne 32 bity zawsze 0x02000000, górne 32 bity to appid.
        let id = rungameid_64(0x9AC34567);
        assert_eq!(id, 0x9AC34567_02000000);
    }
}
