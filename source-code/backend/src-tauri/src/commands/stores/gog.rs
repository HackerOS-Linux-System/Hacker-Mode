use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
pub struct GogProvider;

const GOG_LOGIN_URL: &str = "https://auth.gog.com/auth?client_id=46899977096215655&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code&layout=client2";

#[derive(Debug, Deserialize)]
struct GogdlInstalledGame {
    id: String,
    title: Option<String>,
    install_path: Option<String>,
    #[serde(default)]
    dlcs: Vec<serde_json::Value>,
}

impl GogProvider {
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .join("hacker-mode/gog")
    }
}

impl StoreProvider for GogProvider {
    fn platform(&self) -> Platform {
        Platform::Gog
    }

    fn is_available(&self) -> bool {
        tool_available("gogdl")
    }

    fn is_logged_in(&self) -> bool {
        Self::config_dir().join("auth.json").exists()
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Ok(LoginFlow {
            url: Some(GOG_LOGIN_URL.to_string()),
            instructions: "Zaloguj się na swoje konto GOG.com. Po zalogowaniu strona \
                przekieruje na „embed.gog.com” z parametrem „code” w adresie — Hacker Mode \
                wykryje go automatycznie."
                .to_string(),
            needs_code: true,
        })
    }

    fn login_submit(&self, code: &str) -> Result<(), StoreError> {
        let output = Command::new("gogdl")
            .args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["auth", "--code", code])
            .output()
            .map_err(|e| StoreError::ExecFailed("gogdl auth".into(), e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(StoreError::ExecFailed(
                "gogdl auth".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("gogdl")
            .args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["list-installed"])
            .output()
            .map_err(|e| StoreError::ExecFailed("gogdl".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("GOG".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<GogdlInstalledGame> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("gogdl list-installed".into(), e.to_string()))?;

        let games = parsed
            .into_iter()
            .map(|entry| Game {
                title: entry.title.clone().unwrap_or_else(|| format!("Gra GOG #{}", entry.id)),
                cover_path: find_cover_art(&entry.id),
                id: entry.id,
                platform: Platform::Gog,
                installed: true,
                install_dir: entry.install_path,
                playtime_minutes: None,
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["download", game_id]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["uninstall", game_id]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("gogdl");
        cmd.args(["--auth-config-path"])
            .arg(Self::config_dir())
            .args(["launch", game_id]);
        Ok(cmd)
    }
}

/// GOG wystawia publiczny (nieuwierzytelniony), od dawna stabilny endpoint
/// `api.gog.com/products/<id>` używany przez wiele narzędzi społecznościowych
/// (GOGDB i inne) do pobierania metadanych katalogowych, włącznie z
/// artworkiem. Nie wymaga logowania, więc można go odpytać zawsze — w
/// przeciwieństwie do Epic, gdzie okładki są dostępne tylko z lokalnego
/// cache `legendary` po zalogowaniu.
fn find_cover_art(product_id: &str) -> Option<String> {
    let url = format!("https://api.gog.com/products/{product_id}?expand=images");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent("HackerMode/0.3")
        .build()
        .ok()?;
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    let image_url = crate::cover_cache::find_image_url(&parsed)?;
    // Adresy z GOG API bywają bez schematu (`//images.gog.com/...`).
    let image_url = if image_url.starts_with("//") {
        format!("https:{image_url}")
    } else {
        image_url
    };
    crate::cover_cache::cache_image(&image_url, &format!("gog-{product_id}"))
}
