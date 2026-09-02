use std::path::PathBuf;

// ===================== Bezpieczeństwo zapisu =====================
//
// Dwie proste, niezależne linie obrony przed uszkodzeniem plików
// konfiguracyjnych Steam/Lutrisa, dodane po tym, jak `list_settable_proton_versions`
// miało realny błąd wykryty dopiero researchem (patrz CHANGELOG 0.7.1) —
// skoro parsowanie formatu potrafi się mylić mimo starań, zapis MUSI mieć
// siatkę bezpieczeństwa niezależną od tego, czy patch tekstu akurat jest
// poprawny:
//
// 1. `backup_before_write` — kopia pliku sprzed zmiany, z znacznikiem
//    czasu, żeby dało się ręcznie przywrócić, gdyby coś poszło nie tak.
//    NIE jest to automatyczny rollback (patrz niżej, dlaczego) — to tylko
//    siatka bezpieczeństwa do ręcznego użycia.
// 2. `warn_if_process_running` — Steam/Lutris same piszą do tych plików
//    (przy zamykaniu, przy każdej zmianie ustawień z ich własnego UI);
//    jeśli edytujemy plik, gdy dany program jest uruchomiony, jego
//    kolejny zapis może po prostu NADPISAĆ naszą zmianę (albo odwrotnie —
//    nasza chirurgiczna podmiana tekstu może trafić w plik w trakcie
//    zapisu przez tamten program). Hacker Mode nie ma żadnego sposobu na
//    prawdziwe zablokowanie pliku należącego do INNEGO procesu — jedyna
//    uczciwa opcja to sprawdzić, czy ten proces działa, i OSTRZEC
//    użytkownika przed zapisem, zamiast fałszywie obiecywać bezpieczeństwo,
//    którego nie da się tu zagwarantować.

/// Kopiuje `path` do `<path>.hacker-mode-backup-<unix-timestamp>` przed
/// modyfikacją — jeśli plik źródłowy nie istnieje (np. Lutris jeszcze nie
/// stworzył configu tej gry), nic nie robi (nie ma czego kopiować, to nie
/// błąd). Błąd samego kopiowania jest CELOWO tylko logowany, nie przerywa
/// zapisu — brak backupu nie powinien blokować użytkownika przed zmianą
/// ustawienia, którą chce wykonać; to zapasowa siatka bezpieczeństwa, nie
/// warunek konieczny.
/// Maksymalna liczba kopii zapasowych trzymanych PER plik (`config.vdf`
/// / per-growy YAML Lutrisa) — bez limitu każda zmiana wersji Proton/Wine
/// dokładałaby kolejny plik `*.hacker-mode-backup-<timestamp>` bez
/// końca. 10 wystarcza, żeby cofnąć się o kilka ostatnich zmian, nie
/// zaśmiecając przy tym katalogu w nieskończoność.
const MAX_BACKUPS_PER_FILE: usize = 10;

fn backup_before_write(path: &std::path::Path) {
    if !path.is_file() {
        return;
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = path.with_extension(format!(
        "{}.hacker-mode-backup-{timestamp}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("bak")
    ));
    if let Err(err) = std::fs::copy(path, &backup_path) {
        tracing::warn!(?err, path = %path.display(), "Nie udało się utworzyć kopii zapasowej przed zapisem");
        return;
    }
    tracing::debug!(backup = %backup_path.display(), "Utworzono kopię zapasową przed zapisem");
    prune_old_backups(path);
}

/// Zapisuje `new_content` do `path`, ale TYLKO jeśli plik na dysku wciąż
/// ma DOKŁADNIE taką zawartość jak `expected_content` (ta, na podstawie
/// której `new_content` zostało wyliczone) — prosta, best-effort
/// optymistyczna kontrola współbieżności (compare-and-swap na poziomie
/// pliku), patrz README „Do zrobienia dalej” → „Bezpieczeństwo zapisu
/// plików”.
///
/// WAŻNE ograniczenie: to NIE jest prawdziwa blokada międzyprocesowa.
/// Hacker Mode i tak nie może wymusić, żeby Steam/Lutris respektowały
/// jakikolwiek zamek, którego same nie sprawdzają — te procesy piszą do
/// tych plików bez pytania nikogo o pozwolenie. Ta funkcja łapie
/// KONKRETNY, realny scenariusz wyścigu: Steam/Lutris zapisały plik
/// MIĘDZY naszym odczytem (`content` przekazane wcześniej do
/// `set_steam_compat_tool`/`set_lutris_wine_version`) a naszym zapisem —
/// bez tej kontroli nasz zapis by go po cichu nadpisał, GUBIĄC to, co
/// właśnie zapisał Steam/Lutris (np. świeżo zaktualizowany stan
/// `LastPlayed`/inne pola w tym samym pliku). Z tą kontrolą — odmawiamy
/// zapisu i zwracamy błąd instruujący wywołującego, żeby spróbował
/// ponownie (świeży odczyt + patch + zapis na aktualnej zawartości).
///
/// Nie eliminuje to okna między TYM odczytem-weryfikującym a właściwym
/// zapisem (wciąż mikroskopijne, ale niezerowe) — to fundamentalne
/// ograniczenie zapisu plików bez wsparcia systemu plików dla
/// prawdziwych blokad (`flock` też by nie pomógł, bo Steam/Lutris go nie
/// sprawdzają), ale zawęża okno wyścigu z „cały czas między odczytem
/// przez UI a kliknięciem zapisu przez użytkownika” (może być sekundy
/// czy minuty) do „czas jednego dodatkowego odczytu pliku” (mikrosekundy).
fn write_if_unchanged(path: &std::path::Path, expected_content: &str, new_content: &str) -> Result<(), String> {
    let current = match std::fs::read_to_string(path) {
        Ok(content) => content,
        // Plik nie istniał jeszcze przy pierwszym odczycie (patrz
        // `set_lutris_wine_version`, gdzie brak pliku jest traktowany
        // jako pusta zawartość przez `unwrap_or_default()`) — jeśli
        // WCIĄŻ go nie ma, to zgadza się z tym, co odczytaliśmy
        // wcześniej, więc tworzymy go teraz. Każdy INNY błąd odczytu
        // (uprawnienia itp.) jest prawdziwym błędem, nie „plik pojawił
        // się i zniknął”.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && expected_content.is_empty() => String::new(),
        Err(e) => return Err(format!("Nie udało się ponownie odczytać pliku tuż przed zapisem: {e}")),
    };
    if current != expected_content {
        return Err(
            "Plik zmienił się od momentu odczytu (prawdopodobnie zapisany właśnie przez Steam/Lutris) — spróbuj ponownie."
                .to_string(),
        );
    }
    std::fs::write(path, new_content).map_err(|e| format!("Nie udało się zapisać pliku: {e}"))
}

/// Usuwa najstarsze kopie zapasowe danego pliku ponad
/// `MAX_BACKUPS_PER_FILE` — wywoływane po KAŻDYM udanym
/// `backup_before_write`, żeby katalog nigdy nie urósł ponad ten limit,
/// niezależnie od tego, ile razy użytkownik zmieni wersję Proton/Wine w
/// ciągu życia instalacji Hacker Mode. Błędy usuwania (np. brak
/// uprawnień) są tylko logowane — nieudane posprzątanie starych kopii
/// nie powinno przerywać niczego, co użytkownik faktycznie chciał zrobić
/// (zmienić wersję Proton/Wine).
fn prune_old_backups(original_path: &std::path::Path) {
    let Some(parent) = original_path.parent() else { return };
    let Some(original_name) = original_path.file_name().and_then(|n| n.to_str()) else { return };
    // Prefiks bez rozszerzenia — np. dla `config.vdf` to `config`, żeby
    // dopasować `config.vdf.hacker-mode-backup-<ts>`, a NIE przypadkowo
    // dopasować kopie INNEGO pliku w tym samym katalogu (np. gdyby kiedyś
    // powstał też backup pliku o zbliżonej nazwie).
    let Ok(entries) = std::fs::read_dir(parent) else { return };

    let mut backups: Vec<(std::path::PathBuf, u64)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let marker = format!("{original_name}.hacker-mode-backup-");
            let timestamp_str = name.strip_prefix(&marker)?;
            let timestamp: u64 = timestamp_str.parse().ok()?;
            Some((e.path(), timestamp))
        })
        .collect();

    if backups.len() <= MAX_BACKUPS_PER_FILE {
        return;
    }
    backups.sort_by_key(|(_, ts)| *ts);
    let to_remove = backups.len() - MAX_BACKUPS_PER_FILE;
    for (path, _) in backups.into_iter().take(to_remove) {
        if let Err(err) = std::fs::remove_file(&path) {
            tracing::warn!(?err, path = %path.display(), "Nie udało się usunąć przeterminowanej kopii zapasowej");
        }
    }
}

/// Sprawdza, czy proces o danej nazwie (np. `"steam"`, `"lutris"`) jest
/// aktualnie uruchomiony. Dwuetapowo:
/// 1. `pgrep -x <name>` — dopasowanie DOKŁADNEJ nazwy procesu, szybkie i
///    wystarczające dla typowej natywnej instalacji.
/// 2. Jeśli to nic nie znajdzie, `pgrep -f <name>` — dopasowanie
///    PODCIĄGU w PEŁNEJ linii poleceń, łapiące przypadki, w których
///    nazwa procesu widoczna w `ps` nie jest dokładnie `name` (typowe dla
///    Steam zainstalowanego przez Flatpak, gdzie proces może być
///    opakowany w `bwrap`/`flatpak run` — sama nazwa binarki Steam
///    zwykle i tak pojawia się gdzieś w linii poleceń, nawet jeśli nie
///    jest to nazwa "przednia" procesu widoczna przy `-x`).
/// Fałszywy negatyw jest tu bezpieczniejszy niż fałszywy pozytyw byłby w
/// odwrotną stronę: to i tak tylko ostrzeżenie (patrz wywołania w
/// `set_steam_compat_tool`/`set_lutris_wine_version`), nie blokada, więc
/// konsekwencją przeoczenia jest brak ostrzeżenia, nie zablokowanie
/// funkcji.
fn is_process_running(name: &str) -> bool {
    let exact = std::process::Command::new("pgrep")
        .args(["-x", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if exact {
        return true;
    }
    std::process::Command::new("pgrep")
        .args(["-f", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ===================== Steam =====================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompatToolOption {
    /// Wewnętrzna nazwa narzędzia — w `compatibilitytool.vdf` to KLUCZ
    /// zagnieżdżonego bloku pod `"compat_tools"`, nie osobne pole (patrz
    /// `parse_compat_tool_vdf`). TO jest wartość zapisywana do
    /// `config.vdf` jako `"name"`.
    pub internal_name: String,
    /// Nazwa czytelna dla człowieka (`display_name` z tego samego pliku,
    /// z fallbackiem na nazwę katalogu, gdy plik jej nie podaje).
    pub display_name: String,
}

/// Lista narzędzi kompatybilności, które Hacker Mode może bezpiecznie
/// PRZYPISAĆ grze (patrz moduł-dokumentacja: tylko `compatibilitytools.d/`,
/// nie oficjalny Proton) — czyta `compatibilitytool.vdf` z każdego
/// podkatalogu, ignoruje te bez tego pliku (nie jest to prawidłowe
/// narzędzie kompatybilności, tylko przypadkowy folder).
pub fn list_settable_proton_versions() -> Vec<CompatToolOption> {
    let Some(root) = crate::commands::stores::steam::SteamProvider::find_root() else {
        return Vec::new();
    };
    let dir = root.join("compatibilitytools.d");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| parse_compat_tool_vdf(&e.path().join("compatibilitytool.vdf")))
        .collect()
}

/// Odczyt narzędzia kompatybilności z `compatibilitytool.vdf` —
/// WEWNĘTRZNA nazwa (`internal_name` zapisywane potem do `config.vdf`) to
/// TU NIE osobne pole, tylko klucz zagnieżdżonego bloku pod
/// `"compat_tools"` (pierwsza wcześniejsza wersja tego parsera zakładała
/// błędnie płaskie pole `"internal_name"` — poprawione po zweryfikowaniu
/// prawdziwego formatu w dokumentacji `steam-runtime-tools`, patrz niżej).
/// Prawdziwy kształt pliku (potwierdzone:
/// <https://github.com/xytovl/steam-runtime-tools/blob/feat/openxr/docs/steam-compat-tool-interface.md>,
/// dokumentacja techniczna Steam Runtime, dokładnie ten format czyta sam
/// klient Steam):
/// ```text
/// "compatibilitytools"
/// {
///     "compat_tools"
///     {
///         "GE-Proton9-20"
///         {
///             "install_path"   "."
///             "display_name"   "GE-Proton9-20"
///         }
///     }
/// }
/// ```
/// Zakładamy DOKŁADNIE jeden zagnieżdżony klucz pod `"compat_tools"` na
/// plik — w praktyce tak to wygląda dla każdego znanego narzędzia
/// (Proton-GE i pochodne), a dokumentacja explicite mówi "compat tools
/// can presumably contain one or more", więc gdyby kiedyś jakiś plik miał
/// więcej niż jeden wpis, bierzemy tylko pierwszy zamiast próbować
/// pokazać wszystkie — bezpieczniejsze niż zgadywanie, który jest
/// właściwy.
fn parse_compat_tool_vdf(path: &std::path::Path) -> Option<CompatToolOption> {
    let content = std::fs::read_to_string(path).ok()?;
    let marker_pos = content.find("\"compat_tools\"")?;
    let brace_open = content[marker_pos..].find('{')? + marker_pos;
    let after_brace = &content[brace_open + 1..];

    let internal_name = extract_first_quoted_string(after_brace)?;
    let display_name = extract_quoted_value(after_brace, "display_name").unwrap_or_else(|| internal_name.clone());
    Some(CompatToolOption { internal_name, display_name })
}

/// Zwraca pierwszy cudzysłowiony token w `text` — używane do wyłuskania
/// klucza `"<internal_name>"` zaraz po otwierającej klamrze bloku
/// `"compat_tools"` (patrz `parse_compat_tool_vdf`), gdzie ten klucz nie
/// ma żadnej stałej nazwy do wyszukania przez `extract_quoted_value`
/// (to WŁAŚNIE jest szukana wartość, nie wartość przypisana do
/// znanego klucza).
fn extract_first_quoted_string(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Bardzo minimalny odczyt wartości pod danym kluczem z
/// `compatibilitytool.vdf` — ten sam prosty, linia-po-linii styl parsera
/// VDF co reszta projektu (patrz `steam.rs`), bo to tylko płaski plik
/// klucz-wartość, bez zagnieżdżeń, których szukamy.
fn extract_quoted_value(content: &str, key: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r#""{}"\s*"([^"]*)""#, regex::escape(key))).ok()?;
    re.captures(content).map(|c| c[1].to_string())
}

/// Aktualnie przypisane narzędzie kompatybilności dla danego Steam AppID
/// — działa dla WSZYSTKICH narzędzi (oficjalny Proton, Proton-GE, `none`),
/// bo to tylko odczyt, nie zgadywanie nazwy do zapisu (patrz
/// moduł-dokumentacja). Zwraca `None`, gdy gra w ogóle nie ma jawnego
/// wpisu w `CompatToolMapping` (czyli używa domyślnego ustawienia
/// „Automatyczne wykrywanie” Steam, o którym `config.vdf` po prostu
/// milczy).
pub fn get_steam_compat_tool(appid: &str) -> Option<String> {
    let content = read_config_vdf()?;
    let section = extract_compat_tool_mapping_section(&content)?;
    extract_appid_name(section, appid)
}

/// Przypisuje narzędzie kompatybilności o podanej `internal_name`
/// (WYŁĄCZNIE spośród `list_settable_proton_versions()` — patrz
/// moduł-dokumentacja po uzasadnienie tego ograniczenia) do danego
/// AppID, chirurgicznie podmieniając/wstawiając blok w `config.vdf`.
///
/// Zwraca `Ok(Some(ostrzeżenie))`, gdy zapis się udał, ale Steam był w
/// trakcie uruchomiony (patrz `is_process_running`/moduł-dokumentacja
/// „Bezpieczeństwo zapisu”) — wywołujący (patrz `commands::set_compat_tool`)
/// powinien pokazać to ostrzeżenie użytkownikowi, NIE traktować go jako
/// błąd: zapis i tak się wykonał, backup i tak powstał, tylko wynik może
/// zostać nadpisany przez Steam przy jego następnym własnym zapisie.
pub fn set_steam_compat_tool(appid: &str, internal_name: &str) -> Result<Option<String>, String> {
    let path = config_vdf_path().ok_or("Nie znaleziono katalogu Steam")?;
    let steam_running = is_process_running("steam");
    backup_before_write(&path);
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Nie udało się odczytać config.vdf: {e}"))?;

    let (section_start, section_end) =
        find_compat_tool_mapping_bounds(&content).ok_or("Nie znaleziono sekcji CompatToolMapping w config.vdf")?;
    let section = &content[section_start..section_end];

    let new_section = if let Some((block_start, block_end)) = find_appid_block_bounds(section, appid) {
        // Gra ma już wpis — podmieniamy TYLKO wartość "name" w jej bloku,
        // resztę pól (`config`/`priority`) zostawiamy bez zmian.
        let block = &section[block_start..block_end];
        let patched_block = replace_or_insert_name(block, internal_name);
        format!("{}{}{}", &section[..block_start], patched_block, &section[block_end..])
    } else {
        // Gra nie ma jeszcze wpisu — dopisujemy nowy blok zaraz po
        // otwierającej klamrze sekcji `CompatToolMapping`.
        //
        // Wielkość liter w kluczu "priority" jest niespójna między
        // realnymi przykładami znalezionymi podczas researchu tej funkcji
        // (nowsze źródła: "priority" małą literą; starsze wątki
        // społeczności Steam Deck z lat 2019-2022: "Priority" wielką) —
        // prawdopodobnie parser VDF Steam traktuje klucze bez rozróżniania
        // wielkości liter, więc nie powinno mieć to znaczenia, ale
        // niezweryfikowane na żywej maszynie (patrz README, „Do zrobienia
        // dalej”).
        let insert_at = section.find('{').map(|i| i + 1).ok_or("Nieprawidłowy format sekcji CompatToolMapping (brak klamry otwierającej)")?;
        let new_block = format!(
            "\n\t\t\t\t\t\"{appid}\"\n\t\t\t\t\t{{\n\t\t\t\t\t\t\"name\"\t\t\"{internal_name}\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t\t}}"
        );
        format!("{}{}{}", &section[..insert_at], new_block, &section[insert_at..])
    };

    let patched = format!("{}{}{}", &content[..section_start], new_section, &content[section_end..]);
    write_if_unchanged(&path, &content, &patched)?;

    if steam_running {
        Ok(Some(
            "Steam był uruchomiony podczas zapisu — jeśli Steam zapisze config.vdf przed swoim zamknięciem, może nadpisać tę zmianę. Zamknij Steam i sprawdź ustawienie ponownie.".to_string(),
        ))
    } else {
        Ok(None)
    }
}

fn config_vdf_path() -> Option<PathBuf> {
    Some(crate::commands::stores::steam::SteamProvider::find_root()?.join("config/config.vdf"))
}

fn read_config_vdf() -> Option<String> {
    std::fs::read_to_string(config_vdf_path()?).ok()
}

/// Zwraca podciąg `content` odpowiadający całej sekcji
/// `"CompatToolMapping" { ... }`, WŁĄCZNIE z klamrami — liczy głębokość
/// klamer ręcznie (VDF nie ma cudzysłowionych klamer w wartościach w tym
/// pliku, więc prosty licznik wystarcza, bez potrzeby prawdziwego
/// tokenizera).
fn find_compat_tool_mapping_bounds(content: &str) -> Option<(usize, usize)> {
    let marker = "\"CompatToolMapping\"";
    let marker_pos = content.find(marker)?;
    let brace_open = content[marker_pos..].find('{')? + marker_pos;

    let mut depth = 0i32;
    for (offset, ch) in content[brace_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((marker_pos, brace_open + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_compat_tool_mapping_section(content: &str) -> Option<&str> {
    let (start, end) = find_compat_tool_mapping_bounds(content)?;
    Some(&content[start..end])
}

fn extract_appid_name(section: &str, appid: &str) -> Option<String> {
    let (start, end) = find_appid_block_bounds(section, appid)?;
    extract_quoted_value(&section[start..end], "name")
}

/// Analogiczne do `find_compat_tool_mapping_bounds`, ale dla bloku
/// pojedynczego AppID WEWNĄTRZ sekcji `CompatToolMapping` — zwraca zakres
/// WŁĄCZNIE z cudzysłowionym kluczem `"<appid>"` na początku, żeby
/// `set_steam_compat_tool` mogło podmienić cały blok razem z jego
/// nagłówkiem bez osobnego namierzania obu granic.
fn find_appid_block_bounds(section: &str, appid: &str) -> Option<(usize, usize)> {
    let key = format!("\"{appid}\"");
    let key_pos = section.find(&key)?;
    let brace_open = section[key_pos..].find('{')? + key_pos;

    let mut depth = 0i32;
    for (offset, ch) in section[brace_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((key_pos, brace_open + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn replace_or_insert_name(block: &str, internal_name: &str) -> String {
    let re = regex::Regex::new(r#""name"(\s*)"([^"]*)""#).unwrap();
    if re.is_match(block) {
        re.replace(block, |caps: &regex::Captures| {
            format!("\"name\"{}\"{}\"", &caps[1], internal_name)
        })
        .into_owned()
    } else {
        // Skrajnie nietypowy przypadek (blok bez pola "name" w ogóle) —
        // dopisujemy je zaraz po otwierającej klamrze bloku.
        if let Some(pos) = block.find('{') {
            let mut out = block.to_string();
            out.insert_str(pos + 1, &format!("\n\t\t\t\t\t\t\"name\"\t\t\"{internal_name}\""));
            out
        } else {
            block.to_string()
        }
    }
}

// ===================== Lutris =====================

/// Lista wersji Wine, które Lutris ma zainstalowane przez własny menedżer
/// runnerów — katalog dobrze udokumentowany (community, sam Lutris go tak
/// nazywa w swoim kodzie źródłowym): `~/.local/share/lutris/runners/wine/`,
/// każdy podkatalog to jedna zainstalowana wersja/build.
pub fn list_lutris_wine_versions() -> Vec<String> {
    let dir = dirs::data_dir().unwrap_or_default().join("lutris/runners/wine");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut versions: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    versions.sort();
    versions
}

/// `configpath` (nazwa pliku YAML per gra, BEZ rozszerzenia) dla danego
/// sluga — odczytany z `pga.db` (ta sama baza co czas gry/deinstalacja w
/// `lutris.rs`), bo `configpath` NIE zawsze pokrywa się ze `slug` (Lutris
/// dopisuje sufiks przy kolizji nazw, np. dwie instalacje tej samej gry).
fn resolve_lutris_config_path(slug: &str) -> Option<String> {
    if !crate::commands::stores::tool_available("sqlite3") {
        return None;
    }
    let db_path = crate::commands::stores::lutris::pga_db_path();
    if !db_path.is_file() {
        return None;
    }
    let output = std::process::Command::new("sqlite3")
        .arg(&db_path)
        .arg(format!(
            "SELECT configpath FROM games WHERE slug = '{}' LIMIT 1;",
            slug.replace('\'', "''")
        ))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let configpath = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if configpath.is_empty() {
        None
    } else {
        Some(configpath)
    }
}

fn lutris_game_yaml_path(slug: &str) -> Option<PathBuf> {
    let configpath = resolve_lutris_config_path(slug)?;
    Some(
        dirs::config_dir()
            .unwrap_or_default()
            .join("lutris/games")
            .join(format!("{configpath}.yml")),
    )
}

/// Wersja Wine przypisana do danej gry Lutrisa — `None`, jeśli gra używa
/// domyślnej wersji Lutrisa (plik/sekcja `wine:` po prostu nie ma wtedy
/// klucza `version`) albo jeśli nie udało się w ogóle zlokalizować pliku
/// konfiguracyjnego (patrz `resolve_lutris_config_path`).
pub fn get_lutris_wine_version(slug: &str) -> Option<String> {
    let path = lutris_game_yaml_path(slug)?;
    let content = std::fs::read_to_string(path).ok()?;
    extract_yaml_wine_version(&content)
}

/// Odczyt `version:` WEWNĄTRZ sekcji `wine:` — prosty skan linia po linii
/// zamiast prawdziwego parsera YAML (patrz moduł-dokumentacja): szuka
/// linii `wine:` na poziomie zerowego wcięcia, potem w kolejnych
/// wciętych liniach szuka `version:`, kończy przy pierwszej linii z
/// wcięciem mniejszym lub równym `wine:` (czyli końcu tej sekcji).
fn extract_yaml_wine_version(content: &str) -> Option<String> {
    let mut in_wine_section = false;
    for line in content.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if !in_wine_section {
            if indent == 0 && trimmed.starts_with("wine:") {
                in_wine_section = true;
            }
            continue;
        }
        if indent == 0 && !trimmed.is_empty() {
            // Wyszliśmy z sekcji `wine:` bez znalezienia `version:`.
            return None;
        }
        if let Some(rest) = trimmed.strip_prefix("version:") {
            return Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

/// Ustawia wersję Wine dla danej gry Lutrisa — chirurgiczna podmiana
/// jednej linii (patrz moduł-dokumentacja), z dopisaniem sekcji `wine:`
/// na początku pliku, jeśli jej tam jeszcze nie ma.
///
/// Patrz `set_steam_compat_tool` po wyjaśnienie `Ok(Some(ostrzeżenie))`
/// vs `Ok(None)` — tu ten sam mechanizm, tylko dla procesu `lutris`.
pub fn set_lutris_wine_version(slug: &str, version: &str) -> Result<Option<String>, String> {
    let path = lutris_game_yaml_path(slug).ok_or_else(|| {
        "Nie udało się zlokalizować pliku konfiguracyjnego tej gry w Lutrisie".to_string()
    })?;
    let lutris_running = is_process_running("lutris");
    backup_before_write(&path);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let patched = patch_yaml_wine_version(&content, version);
    write_if_unchanged(&path, &content, &patched)?;

    if lutris_running {
        Ok(Some(
            "Lutris był uruchomiony podczas zapisu — jeśli Lutris zapisze ten plik przed swoim zamknięciem, może nadpisać tę zmianę. Zamknij Lutrisa i sprawdź ustawienie ponownie.".to_string(),
        ))
    } else {
        Ok(None)
    }
}

fn patch_yaml_wine_version(content: &str, version: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    let wine_line_idx = lines.iter().position(|l| {
        let indent = l.len() - l.trim_start().len();
        indent == 0 && l.trim().starts_with("wine:")
    });

    let Some(wine_idx) = wine_line_idx else {
        // Brak sekcji `wine:` w ogóle — dopisujemy ją na końcu pliku.
        lines.push("wine:".to_string());
        lines.push(format!("  version: {version}"));
        return lines.join("\n") + "\n";
    };

    // Szukamy `version:` wewnątrz sekcji, zaczynając zaraz po `wine:`.
    let mut version_idx = None;
    let mut section_end = lines.len();
    for (offset, line) in lines[wine_idx + 1..].iter().enumerate() {
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && !line.trim().is_empty() {
            section_end = wine_idx + 1 + offset;
            break;
        }
        if line.trim_start().starts_with("version:") {
            version_idx = Some(wine_idx + 1 + offset);
        }
    }

    match version_idx {
        Some(idx) => lines[idx] = format!("  version: {version}"),
        None => lines.insert(wine_idx + 1, format!("  version: {version}")),
    }
    let _ = section_end; // tylko do wyznaczenia granicy podczas szukania, nieużywane dalej

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_if_unchanged_writes_when_content_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.vdf");
        std::fs::write(&path, "stara zawartość").unwrap();

        write_if_unchanged(&path, "stara zawartość", "nowa zawartość").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nowa zawartość");
    }

    #[test]
    fn write_if_unchanged_rejects_stale_write() {
        // Symulacja wyścigu: my odczytaliśmy "stara zawartość", ale ZANIM
        // zdążyliśmy zapisać, Steam/Lutris (symulowane tu bezpośrednim
        // `fs::write`) zdążyły dopisać coś swojego. Nasz zapis MUSI zostać
        // odrzucony, żeby nie nadpisać ich zmiany po cichu.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.vdf");
        std::fs::write(&path, "stara zawartość").unwrap();

        // "Steam" dopisuje coś swojego między naszym odczytem a zapisem.
        std::fs::write(&path, "zawartość zapisana przez Steam w międzyczasie").unwrap();

        let result = write_if_unchanged(&path, "stara zawartość", "nasza nowa zawartość");

        assert!(result.is_err());
        // Plik NIE został nadpisany — zawartość zapisana przez "Steam"
        // przetrwała.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "zawartość zapisana przez Steam w międzyczasie"
        );
    }

    #[test]
    fn write_if_unchanged_creates_missing_file_when_expected_empty() {
        // Odpowiednik `set_lutris_wine_version`, gdzie brak pliku przy
        // pierwszym odczycie jest traktowany jak pusta zawartość
        // (`unwrap_or_default()`).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nowa-gra.yml");
        assert!(!path.exists());

        write_if_unchanged(&path, "", "wine:\n  version: proton-ge\n").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "wine:\n  version: proton-ge\n");
    }

    #[test]
    fn backup_before_write_copies_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.vdf");
        std::fs::write(&path, "oryginalna zawartość").unwrap();

        backup_before_write(&path);

        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("hacker-mode-backup"))
            .collect();
        assert_eq!(backups.len(), 1);
        let backup_content = std::fs::read_to_string(backups[0].path()).unwrap();
        assert_eq!(backup_content, "oryginalna zawartość");
        // Oryginał zostaje nietknięty — to kopia, nie przeniesienie.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "oryginalna zawartość");
    }

    #[test]
    fn backup_before_write_missing_source_file_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nieistniejacy.vdf");
        // Nie powinno panikować ani utworzyć żadnego pliku.
        backup_before_write(&path);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn backup_before_write_prunes_old_backups_beyond_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.vdf");
        std::fs::write(&path, "v0").unwrap();

        // Tworzymy więcej kopii niż limit, z jawnie różnymi (rosnącymi)
        // znacznikami czasu — `backup_before_write` samo woła
        // `SystemTime::now()`, więc wywołania w pętli w tym samym
        // milisekundowym oknie mogłyby dać ten sam sekundowy znacznik i
        // nadpisać się nawzajem; budujemy nazwy ręcznie, żeby test był
        // deterministyczny niezależnie od szybkości maszyny.
        for i in 0..(MAX_BACKUPS_PER_FILE + 3) {
            let backup_name = format!("config.vdf.hacker-mode-backup-{i}");
            std::fs::write(dir.path().join(backup_name), format!("wersja {i}")).unwrap();
        }

        prune_old_backups(&path);

        let remaining: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("hacker-mode-backup"))
            .collect();
        assert_eq!(remaining.len(), MAX_BACKUPS_PER_FILE);

        // Zostają NAJNOWSZE (najwyższe indeksy), nie najstarsze.
        let kept_names: Vec<String> = remaining.iter().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert!(kept_names.iter().any(|n| n.ends_with("-9") || n.ends_with(&format!("-{}", MAX_BACKUPS_PER_FILE + 2))));
        assert!(!kept_names.iter().any(|n| n.ends_with("-0")));
    }

    #[test]
    fn prune_old_backups_noop_when_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.vdf");
        std::fs::write(dir.path().join("config.vdf.hacker-mode-backup-1"), "x").unwrap();
        prune_old_backups(&path);
        assert!(dir.path().join("config.vdf.hacker-mode-backup-1").exists());
    }

    #[test]
    fn is_process_running_returns_false_for_definitely_nonexistent_process() {
        assert!(!is_process_running("hacker-mode-test-proces-ktory-na-pewno-nie-istnieje"));
    }

    /// Prawdziwy kształt `compatibilitytool.vdf` (potwierdzony przez
    /// dokumentację `steam-runtime-tools`, patrz cytat przy
    /// `parse_compat_tool_vdf`) — ten test istnieje GŁÓWNIE dlatego, że
    /// pierwsza wersja tego parsera zakładała błędnie płaskie pole
    /// `"internal_name"`, którego w prawdziwym pliku nie ma: nazwa
    /// wewnętrzna to klucz zagnieżdżonego bloku pod `"compat_tools"`.
    #[test]
    fn parses_real_compatibilitytool_vdf_shape() {
        let vdf = r#""compatibilitytools"
{
	"compat_tools"
	{
		"GE-Proton9-20"
		{
			"install_path"		"."
			"display_name"		"GE-Proton9-20"
			"from_oslist"		"windows"
			"to_oslist"		"linux"
		}
	}
}
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("compatibilitytool.vdf");
        std::fs::write(&path, vdf).unwrap();

        let option = parse_compat_tool_vdf(&path).unwrap();
        assert_eq!(option.internal_name, "GE-Proton9-20");
        assert_eq!(option.display_name, "GE-Proton9-20");
    }

    #[test]
    fn parse_compat_tool_vdf_falls_back_to_internal_name_without_display_name() {
        let vdf = r#""compatibilitytools"
{
	"compat_tools"
	{
		"my_compat_tool"
		{
			"install_path"		"."
		}
	}
}
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("compatibilitytool.vdf");
        std::fs::write(&path, vdf).unwrap();

        let option = parse_compat_tool_vdf(&path).unwrap();
        assert_eq!(option.internal_name, "my_compat_tool");
        assert_eq!(option.display_name, "my_compat_tool");
    }

    #[test]
    fn parse_compat_tool_vdf_missing_file_returns_none() {
        assert!(parse_compat_tool_vdf(std::path::Path::new("/nonexistent/compatibilitytool.vdf")).is_none());
    }

    #[test]
    fn extracts_yaml_wine_version() {
        let content = "wine:\n  version: lutris-7.2-x86_64\n  esync: true\ngame:\n  exe: foo.exe\n";
        assert_eq!(extract_yaml_wine_version(content), Some("lutris-7.2-x86_64".to_string()));
    }

    #[test]
    fn extracts_yaml_wine_version_returns_none_without_version_key() {
        let content = "wine:\n  esync: true\ngame:\n  exe: foo.exe\n";
        assert_eq!(extract_yaml_wine_version(content), None);
    }

    #[test]
    fn extracts_yaml_wine_version_returns_none_without_wine_section() {
        let content = "game:\n  exe: foo.exe\n";
        assert_eq!(extract_yaml_wine_version(content), None);
    }

    #[test]
    fn patches_existing_version_line_in_place() {
        let content = "wine:\n  version: old-version\n  esync: true\ngame:\n  exe: foo.exe\n";
        let patched = patch_yaml_wine_version(content, "new-version");
        assert_eq!(extract_yaml_wine_version(&patched), Some("new-version".to_string()));
        // Reszta pliku (sekcja `game:`) zostaje nietknięta.
        assert!(patched.contains("exe: foo.exe"));
        assert!(patched.contains("esync: true"));
    }

    #[test]
    fn inserts_version_line_when_wine_section_exists_without_it() {
        let content = "wine:\n  esync: true\ngame:\n  exe: foo.exe\n";
        let patched = patch_yaml_wine_version(content, "new-version");
        assert_eq!(extract_yaml_wine_version(&patched), Some("new-version".to_string()));
        assert!(patched.contains("esync: true"));
    }

    #[test]
    fn inserts_wine_section_when_missing_entirely() {
        let content = "game:\n  exe: foo.exe\n";
        let patched = patch_yaml_wine_version(content, "new-version");
        assert_eq!(extract_yaml_wine_version(&patched), Some("new-version".to_string()));
        assert!(patched.contains("exe: foo.exe"));
    }

    #[test]
    fn finds_compat_tool_mapping_section_bounds() {
        let vdf = r#""InstallConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"CompatToolMapping"
				{
					"440"
					{
						"name"		"proton_experimental"
						"config"		""
						"priority"		"250"
					}
				}
			}
		}
	}
}
"#;
        let section = extract_compat_tool_mapping_section(vdf).unwrap();
        assert!(section.starts_with("\"CompatToolMapping\""));
        assert_eq!(extract_appid_name(section, "440"), Some("proton_experimental".to_string()));
        assert_eq!(extract_appid_name(section, "999"), None);
    }

    #[test]
    fn replace_or_insert_name_replaces_existing_value() {
        let block = "\"440\"\n{\n\t\"name\"\t\t\"proton_experimental\"\n\t\"config\"\t\t\"\"\n}";
        let patched = replace_or_insert_name(block, "GE-Proton9-20");
        assert!(patched.contains("\"name\"\t\t\"GE-Proton9-20\""));
        assert!(patched.contains("\"config\"\t\t\"\""));
    }

    #[test]
    fn replace_or_insert_name_inserts_when_missing() {
        let block = "\"440\"\n{\n\t\"priority\"\t\t\"250\"\n}";
        let patched = replace_or_insert_name(block, "GE-Proton9-20");
        assert!(patched.contains("\"name\"\t\t\"GE-Proton9-20\""));
        assert!(patched.contains("\"priority\"\t\t\"250\""));
    }
}
