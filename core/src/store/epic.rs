use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

// ── Heroic library JSON structures ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HeroicLibraryFile {
    library: Vec<HeroicGame>,
}

#[derive(Debug, Deserialize)]
struct HeroicGame {
    #[serde(rename = "appName")]
    app_name: String,
    title: String,
    #[serde(rename = "is_installed")]
    is_installed: bool,
    #[serde(rename = "install")]
    install: Option<HeroicInstall>,
    #[serde(rename = "art_cover")]
    art_cover: Option<String>,
    #[serde(rename = "art_background")]
    art_background: Option<String>,
    #[serde(rename = "developer")]
    developer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeroicInstall {
    #[serde(rename = "install_path")]
    install_path: Option<String>,
    #[serde(rename = "executable")]
    executable: Option<String>,
    #[serde(rename = "disk_size")]
    disk_size: Option<i64>,
    #[serde(rename = "version")]
    version: Option<String>,
}

// ── Legendary installed.json structures ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LegendaryInstalled {
    #[serde(flatten)]
    games: std::collections::HashMap<String, LegendaryGame>,
}

#[derive(Debug, Deserialize)]
struct LegendaryGame {
    title: String,
    #[serde(rename = "install_path")]
    install_path: String,
    #[serde(rename = "executable")]
    executable: String,
    #[serde(rename = "disk_size")]
    disk_size: Option<i64>,
    developer: Option<String>,
}

// ── Importer ─────────────────────────────────────────────────────────────────

pub struct EpicImporter {
    heroic_config: PathBuf,
    legendary_config: PathBuf,
}

impl EpicImporter {
    pub fn new() -> Self {
        let config = dirs::config_dir().unwrap_or_default();
        Self {
            heroic_config: config.join("heroic"),
            legendary_config: config.join("legendary"),
        }
    }

    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        let mut count = 0usize;
        count += self.import_from_heroic(db).await.unwrap_or_else(|e| {
            warn!("Heroic import failed: {e}");
            0
        });
        count += self.import_from_legendary(db).await.unwrap_or_else(|e| {
            warn!("Legendary import failed: {e}");
            0
        });
        info!("Epic import complete: {count} games");
        Ok(count)
    }

    async fn import_from_heroic(&self, db: &Database) -> Result<usize> {
        let lib_path = self.heroic_config.join("lib-cache/legendary_library.json");
        if !lib_path.exists() {
            debug!("No Heroic library cache at {}", lib_path.display());
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&lib_path).await
            .with_context(|| format!("Reading {}", lib_path.display()))?;
        let parsed: HeroicLibraryFile = serde_json::from_str(&content)?;

        let mut count = 0usize;
        for hg in parsed.library {
            let row = heroic_game_to_row(&hg);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
            debug!("Imported from Heroic: {} ({})", hg.title, hg.app_name);
        }
        Ok(count)
    }

    async fn import_from_legendary(&self, db: &Database) -> Result<usize> {
        let installed_path = self.legendary_config.join("installed.json");
        if !installed_path.exists() {
            debug!("No Legendary installed.json at {}", installed_path.display());
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&installed_path).await?;
        let parsed: LegendaryInstalled = serde_json::from_str(&content)?;

        let mut count = 0usize;
        for (app_name, lg) in parsed.games {
            let row = legendary_game_to_row(&app_name, &lg);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
            debug!("Imported from Legendary: {} ({})", lg.title, app_name);
        }
        Ok(count)
    }
}

fn heroic_game_to_row(hg: &HeroicGame) -> GameRow {
    let mut row = GameRow::new(
        &hg.app_name,
        StoreSource::Epic,
        &hg.title,
        &RunnerType::Heroic { app_name: hg.app_name.clone() },
    );
    row.developer = hg.developer.clone();
    row.is_installed = hg.is_installed;
    if let Some(ref inst) = hg.install {
        row.install_path = inst.install_path.clone();
        row.executable   = inst.executable.clone();
        row.size_bytes   = inst.disk_size;
    }
    row.cover_path = hg.art_cover.clone();
    row.hero_path  = hg.art_background.clone();
    row
}

fn legendary_game_to_row(app_name: &str, lg: &LegendaryGame) -> GameRow {
    let mut row = GameRow::new(
        app_name,
        StoreSource::Epic,
        &lg.title,
        &RunnerType::Heroic { app_name: app_name.to_owned() },
    );
    row.developer    = lg.developer.clone();
    row.install_path = Some(lg.install_path.clone());
    row.executable   = Some(lg.executable.clone());
    row.size_bytes   = lg.disk_size;
    row.is_installed = Path::new(&lg.install_path).exists();
    row
}

// ── Launcher ─────────────────────────────────────────────────────────────────

pub async fn launch_legendary(app_name: &str) -> Result<tokio::process::Child> {
    info!("Launching Epic game '{}' via Legendary", app_name);
    let child = Command::new("legendary")
        .args(["launch", app_name])
        .spawn()
        .context("legendary not found – install it or use Heroic")?;
    Ok(child)
}

pub async fn launch_heroic(app_name: &str) -> Result<tokio::process::Child> {
    info!("Launching Epic game '{}' via Heroic", app_name);
    // Heroic exposes a URL scheme: heroic://launch/<app_name>
    let child = Command::new("xdg-open")
        .arg(format!("heroic://launch/{app_name}"))
        .spawn()
        .context("xdg-open not found")?;
    Ok(child)
}

// ── OAuth2 ────────────────────────────────────────────────────────────────────

/// Returns the URL the WebView should navigate to for Epic login.
/// After the user authenticates, Epic redirects to a localhost callback
/// which WRY intercepts via navigation policy.
pub fn oauth_start_url() -> String {
    // Epic uses a custom OAuth flow; Legendary's auth URL
    "https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9%26responseType%3Dcode".to_owned()
}

/// Called by the WebView navigation handler when it sees the redirect URL.
/// Extracts the auth code and exchanges it for tokens via Legendary.
pub async fn handle_oauth_redirect(redirect_url: &str) -> Result<()> {
    use url::Url;
    let parsed = Url::parse(redirect_url)?;
    let code = parsed.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .context("No code in redirect URL")?;

    info!("Epic OAuth code received, exchanging via legendary auth");
    let status = Command::new("legendary")
        .args(["auth", "--code", &code])
        .status()
        .await
        .context("legendary auth failed")?;

    if !status.success() {
        anyhow::bail!("legendary auth exited with {}", status);
    }
    info!("Epic auth complete");
    Ok(())
}
