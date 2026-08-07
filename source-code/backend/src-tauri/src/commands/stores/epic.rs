use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::process::Command;

#[derive(Default)]
pub struct EpicProvider;

const EPIC_LOGIN_URL: &str = "https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9%26responseType%3Dcode";

/// Odpowiada (najbliżej jak się dało odtworzyć bez dostępu do zainstalowanej
/// wersji `legendary`) polom zwracanym przez `legendary list-installed --json`.
/// Pola nieznane/nieużywane są ignorowane dzięki `#[serde(default)]` na
/// poziomie `Option`.
#[derive(Debug, Deserialize)]
struct LegendaryInstalledGame {
    app_name: String,
    title: Option<String>,
    install_path: Option<String>,
    version: Option<String>,
    executable: Option<String>,
    #[serde(default)]
    is_dlc: bool,
}

impl StoreProvider for EpicProvider {
    fn platform(&self) -> Platform {
        Platform::Epic
    }

    fn is_available(&self) -> bool {
        tool_available("legendary")
    }

    fn is_logged_in(&self) -> bool {
        Command::new("legendary")
            .args(["status", "--json"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Ok(LoginFlow {
            url: Some(EPIC_LOGIN_URL.to_string()),
            instructions: "Zaloguj się na swoje konto Epic Games. Po zalogowaniu strona \
                pokaże krótki kod JSON — skopiuj wartość pola „authorizationCode” i wklej ją \
                w Hacker Mode."
                .to_string(),
            needs_code: true,
        })
    }

    fn login_submit(&self, code: &str) -> Result<(), StoreError> {
        let output = Command::new("legendary")
            .args(["auth", "--code", code])
            .output()
            .map_err(|e| StoreError::ExecFailed("legendary auth".into(), e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(StoreError::ExecFailed(
                "legendary auth".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("legendary")
            .args(["list-installed", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("legendary".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("Epic Games".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<LegendaryInstalledGame> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("legendary list-installed".into(), e.to_string()))?;

        let games = parsed
            .into_iter()
            .filter(|g| !g.is_dlc)
            .map(|entry| {
                let cover_path = find_cover_art(&entry.app_name);
                Game {
                    title: entry.title.clone().unwrap_or_else(|| entry.app_name.clone()),
                    id: entry.app_name,
                    platform: Platform::Epic,
                    installed: true,
                    install_dir: entry.install_path,
                    cover_path,
                    playtime_minutes: None,
                }
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["install", game_id, "-y"]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["uninstall", game_id, "-y"]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("legendary");
        cmd.args(["launch", game_id]);
        Ok(cmd)
    }
}

/// `legendary` cache'uje pełne metadane katalogowe (włącznie z URL-ami
/// artworku) pod `~/.config/legendary/metadata/<app_name>.json` po każdym
/// `legendary list`/`legendary info` — czytamy ten plik, jeśli istnieje,
/// zamiast odpytywać API Epica bezpośrednio (które wymaga uwierzytelnienia
/// niepublicznym tokenem). Jeśli plik nie istnieje (użytkownik nigdy nie
/// odświeżył katalogu), po prostu brak okładki — nie jest to błąd krytyczny.
fn find_cover_art(app_name: &str) -> Option<String> {
    let metadata_path = dirs::config_dir()?
        .join("legendary/metadata")
        .join(format!("{app_name}.json"));
    let content = std::fs::read_to_string(metadata_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let url = crate::cover_cache::find_image_url(&parsed)?;
    crate::cover_cache::cache_image(&url, &format!("epic-{app_name}"))
}
