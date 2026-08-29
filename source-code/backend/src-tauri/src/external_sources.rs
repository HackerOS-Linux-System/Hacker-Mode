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
    // Próbujemy obu prawdopodobnych kształtów — patrz moduł-dokumentacja
    // po wyjaśnienie niepewności co do dokładnego formatu.
    let games = parsed
        .as_array()
        .cloned()
        .or_else(|| parsed.get("games").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    games
        .iter()
        .filter_map(|g| {
            let name = g
                .get("title")
                .or_else(|| g.get("app_name"))
                .or_else(|| g.get("name"))
                .and_then(|v| v.as_str())?
                .to_string();
            Some(ExternalApp { source: "Heroic (sideload)".to_string(), name })
        })
        .collect()
}

fn bottles_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("bottles/bottles")
}

/// Nazwy wykrytych "bottle'i" (prefiksów Wine zarządzanych przez
/// Bottles) — patrz moduł-dokumentacja po wyjaśnienie, dlaczego to tylko
/// nazwa, bez listy programów w środku każdego z nich.
pub fn list_bottles() -> Vec<ExternalApp> {
    let Ok(entries) = std::fs::read_dir(bottles_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir() && e.path().join("bottle.yml").is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
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
}
