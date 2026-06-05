use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

// ── Heroic GOG library ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HeroicGogLibrary {
    library: Vec<HeroicGogGame>,
}

#[derive(Debug, Deserialize)]
struct HeroicGogGame {
    #[serde(rename = "appName")]
    app_name: String,
    title: String,
    #[serde(rename = "is_installed", default)]
    is_installed: bool,
    #[serde(rename = "install")]
    install: Option<HeroicGogInstall>,
    #[serde(rename = "art_cover")]
    art_cover: Option<String>,
    #[serde(rename = "art_background")]
    art_background: Option<String>,
    developer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeroicGogInstall {
    #[serde(rename = "install_path")]
    install_path: Option<String>,
    executable: Option<String>,
    #[serde(rename = "disk_size")]
    disk_size: Option<i64>,
}

pub struct GogImporter {
    heroic_config: PathBuf,
}

impl GogImporter {
    pub fn new() -> Self {
        let config = dirs::config_dir().unwrap_or_default();
        Self { heroic_config: config.join("heroic") }
    }

    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        let count = self.import_from_heroic(db).await.unwrap_or_else(|e| {
            warn!("Heroic GOG import failed: {e}"); 0
        });
        info!("GOG import complete: {count} games");
        Ok(count)
    }

    async fn import_from_heroic(&self, db: &Database) -> Result<usize> {
        let lib_path = self.heroic_config.join("lib-cache/gog_library.json");
        if !lib_path.exists() {
            debug!("No Heroic GOG cache at {}", lib_path.display());
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&lib_path).await?;
        let parsed: HeroicGogLibrary = serde_json::from_str(&content)
            .with_context(|| format!("Parsing {}", lib_path.display()))?;

        let mut count = 0;
        for hg in parsed.library {
            let row = heroic_gog_to_row(&hg);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
        }
        Ok(count)
    }
}

fn heroic_gog_to_row(hg: &HeroicGogGame) -> GameRow {
    let mut row = GameRow::new(
        &hg.app_name,
        StoreSource::Gog,
        &hg.title,
        &RunnerType::Heroic { app_name: hg.app_name.clone() },
    );
    row.developer    = hg.developer.clone();
    row.is_installed = hg.is_installed;
    row.cover_path   = hg.art_cover.clone();
    row.hero_path    = hg.art_background.clone();
    if let Some(ref inst) = hg.install {
        row.install_path = inst.install_path.clone();
        row.executable   = inst.executable.clone();
        row.size_bytes   = inst.disk_size;
    }
    row
}

pub async fn launch_heroic(app_name: &str) -> Result<tokio::process::Child> {
    info!("Launching GOG game '{}' via Heroic", app_name);
    Command::new("xdg-open")
        .arg(format!("heroic://launch/{app_name}"))
        .spawn()
        .context("xdg-open not found")
}

pub fn oauth_start_url() -> String {
    "https://auth.gog.com/auth?client_id=46899977096215655&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code&layout=client2".to_owned()
}

pub async fn handle_oauth_redirect(redirect_url: &str) -> Result<()> {
    if redirect_url.contains("embed.gog.com/on_login_success") {
        info!("GOG OAuth redirect detected");
        let status = Command::new("heroic")
            .args(["--no-gui", "login", "--store", "gog"])
            .status()
            .await
            .context("heroic login failed")?;
        if !status.success() {
            anyhow::bail!("heroic gog login exited with {}", status);
        }
    }
    Ok(())
}
