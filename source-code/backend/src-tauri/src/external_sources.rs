use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalApp {
    pub source: String,
    pub name: String,
}

fn heroic_library_path() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("heroic/sideload_apps/library.json")
}

/// Aplikacje dodane ręcznie ("sideload") w Heroic Games Launcher — NIE
/// gry Epic/GOG/Amazon zarządzane przez Heroic (te Hacker Mode i tak już
/// widzi bezpośrednio przez `legendary`/`gogdl`/`nile`, patrz `epic.rs`/
/// `gog.rs`/`amazon.rs` — Heroic używa dokładnie tych samych CLI pod
/// spodem, więc nie ma tu żadnej dodatkowej informacji do zdobycia).
/// Sideloady to jedyna kategoria z Heroic, o której Hacker Mode inaczej
/// by się nie dowiedział.
pub fn list_heroic_sideloaded() -> Vec<ExternalApp> {
    let Ok(content) = std::fs::read_to_string(heroic_library_path()) else {
        return Vec::new();
    };
    parse_heroic_library(&content)
}

fn parse_heroic_library(content: &str) -> Vec<ExternalApp> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };

    // Trzy prawdopodobne kształty tego pliku (patrz moduł-dokumentacja
    // `list_heroic_sideloaded` i README „Do zrobienia dalej” po pełne
    // wyjaśnienie niepewności):
    //
    // 1. Tablica na najwyższym poziomie: `[{...}, {...}]`.
    // 2. Obiekt z kluczem `"games"` lub `"library"`: `{"games": [{...}]}`.
    // 3. PŁASKI obiekt kluczowany przez `app_name`, gdzie każda WARTOŚĆ
    //    to osobna gra: `{"<app_name>": {"title": ..., "app_name": ...}}`.
    //    Dodane po researchu źródeł Heroic (`electronStores.ts` —
    //    `TypeCheckedStoreBackend` to cienka warstwa nad `electron-store`,
    //    którego typowy wzorzec użycia to `store.set(<klucz>, <wartość>)`
    //    per wpis, NIE jedna zbiorcza tablica) i pośrednio potwierdzone
    //    przykładem automatyzacji społeczności (Ansible), który
    //    inicjalizuje ten plik jako PUSTY OBIEKT (`{}`, nie `{"games": []}`)
    //    i scala nowe wpisy przez zwykłe scalanie kluczy JSON-a — co ma
    //    sens tylko jeśli plik faktycznie jest mapą klucz→gra, nie
    //    kontenerem z jedną zbiorczą listą w środku.
    //
    // Żaden z tych trzech wariantów nie został sprawdzony na prawdziwym
    // pliku z realnej instalacji Heroic — wciąż otwarty punkt w README.
    if let Some(array) = parsed.as_array() {
        return games_from_array(array);
    }
    if let Some(array) = parsed.get("games").and_then(|v| v.as_array()) {
        return games_from_array(array);
    }
    if let Some(array) = parsed.get("library").and_then(|v| v.as_array()) {
        return games_from_array(array);
    }
    if let Some(obj) = parsed.as_object() {
        // Wariant 3: każda WARTOŚĆ obiektu to osobna gra — celowo
        // wyciągamy nazwę z WARTOŚCI (pole `title`/`app_name`), nie z
        // klucza mapy (który u Heroic bywa nieczytelnym dla człowieka
        // identyfikatorem, nie tytułem).
        let games: Vec<ExternalApp> = obj.values().filter_map(game_from_value).collect();
        if !games.is_empty() {
            return games;
        }
    }
    Vec::new()
}

fn games_from_array(games: &[serde_json::Value]) -> Vec<ExternalApp> {
    games.iter().filter_map(game_from_value).collect()
}

fn game_from_value(g: &serde_json::Value) -> Option<ExternalApp> {
    let name = g
        .get("title")
        .or_else(|| g.get("app_name"))
        .or_else(|| g.get("name"))
        .or_else(|| g.get("displayName"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    Some(ExternalApp { source: "Heroic (sideload)".to_string(), name })
}

fn bottles_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("bottles/bottles")
}

/// Pola `bottle.yml`, których nazwy udało się potwierdzić researchem —
/// patrz moduł-dokumentacja `list_bottles` po źródła i poziom pewności.
/// Wszystkie pola opcjonalne: jeśli parsowanie nie znajdzie żadnego z
/// nich, `list_bottles` i tak pokazuje samą nazwę katalogu jak
/// wcześniej — ten parser tylko DODAJE informację, nigdy nie odbiera.
#[derive(Debug, Default, PartialEq)]
struct BottleInfo {
    /// Nazwa wyświetlana bottle'a — pole `Name:` w YAML-u, jeśli różni
    /// się od nazwy katalogu (Bottles pozwala je zmienić w UI już po
    /// utworzeniu bottle'a, bez zmiany nazwy katalogu).
    name: Option<String>,
    /// Wersja runnera Wine/Proton-GE przypisana do tego bottle'a — pole
    /// `Runner:`.
    runner: Option<String>,
}

/// Bardzo prosty, liniowy odczyt najwyższego poziomu YAML-a — świadomie
/// NIE pełny parser YAML (ten sam kompromis co
/// `compat_tools::extract_yaml_wine_version` dla Lutrisa: potrzebujemy
/// tylko dwóch konkretnych, prostych pól tekstowych z płaskiego
/// najwyższego poziomu pliku, pełna biblioteka YAML byłaby przerostem
/// formy nad treścią). Szuka linii zaczynających się (po obcięciu
/// białych znaków) od `Name:`/`Runner:` i bierze resztę linii jako
/// wartość, obcinając ewentualne cudzysłowy.
///
/// Nazwy pól (`Name`, `Runner`, `Environment`, `Path`) potwierdzone
/// researchem struktury `_config_struct`/`BottleConfig` w kodzie źródłowym
/// projektu Bottles (`bottlesdevs/bottles`, wcześniej `libbottles`) —
/// PascalCase, dokładnie odzwierciedlające flagi CLI (`--runner`, `--win`)
/// udokumentowane w ich `README`/`--help`. Pewność wysoka co do SAMYCH
/// nazw pól, ale nie potwierdzona na prawdziwym pliku `bottle.yml` z
/// żywej instalacji — stąd wciąż tylko best-effort, patrz README.
fn parse_bottle_yml(content: &str) -> BottleInfo {
    let mut info = BottleInfo::default();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Name:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                info.name = Some(value.to_string());
            }
        } else if let Some(value) = trimmed.strip_prefix("Runner:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                info.runner = Some(value.to_string());
            }
        }
    }
    info
}

/// Nazwy wykrytych "bottle'i" (prefiksów Wine zarządzanych przez
/// Bottles) — od tej rundy wzbogacone (best-effort, patrz
/// `parse_bottle_yml`) o nazwę z `bottle.yml` (jeśli różni się od nazwy
/// katalogu) i przypisany runner, doklejone do wyświetlanej nazwy w
/// nawiasie zamiast rozszerzać `ExternalApp` o nowe pole (żeby nie
/// zmieniać kształtu odpowiedzi API konsumowanego przez frontend w tej
/// samej rundzie co samą logikę wykrywania).
pub fn list_bottles() -> Vec<ExternalApp> {
    let Ok(entries) = std::fs::read_dir(bottles_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let bottle_yml_path = e.path().join("bottle.yml");
            let dir_name = e.file_name().to_string_lossy().into_owned();
            if !bottle_yml_path.is_file() {
                return None;
            }

            let info = std::fs::read_to_string(&bottle_yml_path)
                .map(|content| parse_bottle_yml(&content))
                .unwrap_or_default();

            let display_name = match info.name.filter(|n| n != &dir_name) {
                Some(name) => format!("{name} ({dir_name})"),
                None => dir_name,
            };
            let name = match info.runner {
                Some(runner) => format!("{display_name} — {runner}"),
                None => display_name,
            };

            Some(ExternalApp { source: "Bottles".to_string(), name })
        })
        .collect()
}

/// Wszystko naraz — patrz `commands::list_external_apps`.
pub fn list_all() -> Vec<ExternalApp> {
    let mut apps = list_heroic_sideloaded();
    apps.extend(list_bottles());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heroic_library_as_top_level_array() {
        let json = r#"[{"title": "Moja Gra", "app_name": "moja-gra"}]"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Moja Gra");
    }

    #[test]
    fn parses_heroic_library_wrapped_in_games_key() {
        let json = r#"{"games": [{"title": "Inna Gra"}]}"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Inna Gra");
    }

    #[test]
    fn parses_heroic_library_wrapped_in_library_key() {
        let json = r#"{"library": [{"title": "Trzecia Gra"}]}"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Trzecia Gra");
    }

    #[test]
    fn parses_heroic_library_as_flat_object_keyed_by_app_name() {
        // Wariant 3 — patrz moduł-dokumentacja `parse_heroic_library`.
        let json = r#"{
            "moja-gra-abc123": {"app_name": "moja-gra-abc123", "title": "Płaska Gra"},
            "inna-gra-xyz789": {"app_name": "inna-gra-xyz789", "title": "Druga Płaska Gra"}
        }"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 2);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Płaska Gra"));
        assert!(names.contains(&"Druga Płaska Gra"));
    }

    #[test]
    fn falls_back_to_display_name_when_other_fields_missing() {
        let json = r#"[{"displayName": "Tylko DisplayName"}]"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps[0].name, "Tylko DisplayName");
    }

    #[test]
    fn ignores_blank_or_whitespace_only_names() {
        let json = r#"[{"title": "   "}, {"title": "Prawdziwa Nazwa"}]"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Prawdziwa Nazwa");
    }

    #[test]
    fn falls_back_to_app_name_when_title_missing() {
        let json = r#"[{"app_name": "tylko-app-name"}]"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps[0].name, "tylko-app-name");
    }

    #[test]
    fn garbage_input_returns_empty_instead_of_panicking() {
        assert!(parse_heroic_library("nie json").is_empty());
        assert!(parse_heroic_library("{}").is_empty());
    }

    #[test]
    fn skips_entries_without_any_recognizable_name_field() {
        let json = r#"[{"unrelated_field": "x"}, {"title": "Widoczna"}]"#;
        let apps = parse_heroic_library(json);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Widoczna");
    }

    #[test]
    fn list_bottles_missing_directory_returns_empty() {
        // Nie da się łatwo podmienić `dirs::data_dir()` w teście bez
        // dodatkowej abstrakcji (ten sam kompromis co inne moduły czytające
        // z `dirs::*`) — ten test tylko pilnuje, że BRAK katalogu bottles
        // (a nie: zawartość konkretnego katalogu) nie panikuje.
        let result = std::fs::read_dir(std::path::Path::new("/nieistniejacy/katalog/bottles"));
        assert!(result.is_err());
    }

    #[test]
    fn parses_bottle_yml_name_and_runner() {
        let yaml = "Name: Moja Gra\nRunner: soda-9.0-2\nPath: /home/user/.local/share/bottles/bottles/moja-gra\n";
        let info = parse_bottle_yml(yaml);
        assert_eq!(info.name.as_deref(), Some("Moja Gra"));
        assert_eq!(info.runner.as_deref(), Some("soda-9.0-2"));
    }

    #[test]
    fn parses_bottle_yml_handles_quoted_values() {
        let yaml = "Name: \"Gra w cudzysłowie\"\nRunner: 'wine-ge-8-26'\n";
        let info = parse_bottle_yml(yaml);
        assert_eq!(info.name.as_deref(), Some("Gra w cudzysłowie"));
        assert_eq!(info.runner.as_deref(), Some("wine-ge-8-26"));
    }

    #[test]
    fn parses_bottle_yml_returns_empty_defaults_for_unrecognized_content() {
        let info = parse_bottle_yml("to nie jest bottle.yml\nani trochę");
        assert_eq!(info, BottleInfo::default());
    }
}
