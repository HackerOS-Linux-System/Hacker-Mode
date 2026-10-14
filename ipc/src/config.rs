use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Labwc,
    Comphwde,
}

impl Compositor {
    pub const DEFAULT: Compositor = Compositor::Labwc;

    pub fn as_str(self) -> &'static str {
        match self {
            Compositor::Labwc => "labwc",
            Compositor::Comphwde => "comphwde",
        }
    }

    /// Case-insensitive; unknown values yield `None` (the caller falls
    /// back to the default rather than guessing).
    pub fn parse(value: &str) -> Option<Compositor> {
        match value.trim().to_ascii_lowercase().as_str() {
            "labwc" => Some(Compositor::Labwc),
            "comphwde" => Some(Compositor::Comphwde),
            _ => None,
        }
    }
}

fn config_home() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => PathBuf::from(home).join(".config"),
        _ => PathBuf::from("."),
    }
}

/// `~/.config/Hacker-Mode/config.json`
pub fn config_path() -> PathBuf {
    config_home().join("Hacker-Mode").join("config.json")
}

/// `~/.config/hacker-mode/config.json` (location used before v1.1).
pub fn legacy_config_path() -> PathBuf {
    config_home().join("hacker-mode").join("config.json")
}

/// Extracts the `"compositor"` value from JSON text. Unknown/invalid
/// content yields `None`.
pub fn compositor_from_json(json: &str) -> Option<Compositor> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    Compositor::parse(value.get("compositor")?.as_str()?)
}

fn compositor_from_file(path: &Path) -> Option<Compositor> {
    compositor_from_json(&std::fs::read_to_string(path).ok()?)
}

/// Full resolution, see module docs.
pub fn resolve_compositor() -> Compositor {
    if let Ok(env) = std::env::var("HACKER_MODE_COMPOSITOR") {
        if let Some(c) = Compositor::parse(&env) {
            return c;
        }
    }
    compositor_from_file(&config_path())
        .or_else(|| compositor_from_file(&legacy_config_path()))
        .unwrap_or(Compositor::DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_labwc() {
        assert_eq!(Compositor::DEFAULT, Compositor::Labwc);
    }

    #[test]
    fn parse_is_case_insensitive_and_strict() {
        assert_eq!(Compositor::parse(" LabWC "), Some(Compositor::Labwc));
        assert_eq!(Compositor::parse("comphwde"), Some(Compositor::Comphwde));
        assert_eq!(Compositor::parse("sway"), None);
        assert_eq!(Compositor::parse(""), None);
    }

    #[test]
    fn json_extraction() {
        assert_eq!(compositor_from_json(r#"{"compositor":"comphwde","x":1}"#), Some(Compositor::Comphwde));
        assert_eq!(compositor_from_json(r#"{"compositor":"nope"}"#), None);
        assert_eq!(compositor_from_json(r#"{"theme":"dark"}"#), None);
        assert_eq!(compositor_from_json("not json"), None);
    }
}
