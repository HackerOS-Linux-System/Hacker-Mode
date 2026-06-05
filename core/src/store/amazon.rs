use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

// ── Nile library.json ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct NileLibrary {
    games: Vec<NileGame>,
}

#[derive(Debug, Deserialize)]
struct NileGame {
    id: String,
    product: NileProduct,
    #[serde(default)]
    state: Option<NileState>,
}

#[derive(Debug, Deserialize)]
struct NileProduct {
    id: String,
    title: String,
    #[serde(rename = "iconUrl")]
    icon_url: Option<String>,
    #[serde(rename = "backgroundUrl1")]
    background_url: Option<String>,
    publisher: Option<String>,
    developer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NileState {
    installed: Option<NileInstalled>,
}

#[derive(Debug, Deserialize)]
struct NileInstalled {
    path: String,
    executable: String,
    #[serde(rename = "diskSize")]
    disk_size: Option<i64>,
}

// ── Heroic Amazon library ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HeroicAmazonLibrary {
    library: Vec<HeroicAmazonGame>,
}

#[derive(Debug, Deserialize)]
struct HeroicAmazonGame {
    #[serde(rename = "appName")]
    app_name: String,
    title: String,
    #[serde(rename = "is_installed")]
    is_installed: bool,
    #[serde(rename = "art_cover")]
    art_cover: Option<String>,
    #[serde(rename = "install")]
    install: Option<HeroicAmazonInstall>,
}

#[derive(Debug, Deserialize)]
struct HeroicAmazonInstall {
    #[serde(rename = "install_path")]
    install_path: Option<String>,
    executable: Option<String>,
    #[serde(rename = "disk_size")]
    disk_size: Option<i64>,
}

// ── Importer ─────────────────────────────────────────────────────────────────

pub struct AmazonImporter {
    nile_config: PathBuf,
    heroic_config: PathBuf,
}

impl AmazonImporter {
    pub fn new() -> Self {
        let config = dirs::config_dir().unwrap_or_default();
        Self {
            nile_config:   config.join("Nile"),
            heroic_config: config.join("heroic"),
        }
    }

    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        let mut count = 0;
        count += self.import_from_nile(db).await.unwrap_or_else(|e| {
            warn!("Nile import failed: {e}"); 0
        });
        count += self.import_from_heroic(db).await.unwrap_or_else(|e| {
            warn!("Heroic Amazon import failed: {e}"); 0
        });
        info!("Amazon import complete: {count} games");
        Ok(count)
    }

    async fn import_from_nile(&self, db: &Database) -> Result<usize> {
        let lib_path = self.nile_config.join("library.json");
        if !lib_path.exists() {
            debug!("No Nile library at {}", lib_path.display());
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&lib_path).await?;
        let parsed: NileLibrary = serde_json::from_str(&content)?;

        let mut count = 0;
        for ng in parsed.games {
            let row = nile_to_row(&ng);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
        }
        Ok(count)
    }

    async fn import_from_heroic(&self, db: &Database) -> Result<usize> {
        let lib_path = self.heroic_config.join("lib-cache/amazon_library.json");
        if !lib_path.exists() {
            debug!("No Heroic Amazon cache at {}", lib_path.display());
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&lib_path).await?;
        let parsed: HeroicAmazonLibrary = serde_json::from_str(&content)?;

        let mut count = 0;
        for hg in parsed.library {
            let row = heroic_amazon_to_row(&hg);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
        }
        Ok(count)
    }
}

fn nile_to_row(ng: &NileGame) -> GameRow {
    let is_installed = ng.state.as_ref()
        .and_then(|s| s.installed.as_ref())
        .map(|i| std::path::Path::new(&i.path).exists())
        .unwrap_or(false);

    let mut row = GameRow::new(
        &ng.product.id,
        StoreSource::Amazon,
        &ng.product.title,
        &RunnerType::Nile { id: ng.id.clone() },
    );
    row.developer    = ng.product.developer.clone();
    row.publisher    = ng.product.publisher.clone();
    row.cover_path   = ng.product.icon_url.clone();
    row.hero_path    = ng.product.background_url.clone();
    row.is_installed = is_installed;

    if let Some(inst) = ng.state.as_ref().and_then(|s| s.installed.as_ref()) {
        row.install_path = Some(inst.path.clone());
        row.executable   = Some(inst.executable.clone());
        row.size_bytes   = inst.disk_size;
    }
    row
}

fn heroic_amazon_to_row(hg: &HeroicAmazonGame) -> GameRow {
    let mut row = GameRow::new(
        &hg.app_name,
        StoreSource::Amazon,
        &hg.title,
        &RunnerType::Heroic { app_name: hg.app_name.clone() },
    );
    row.cover_path   = hg.art_cover.clone();
    row.is_installed = hg.is_installed;
    if let Some(ref inst) = hg.install {
        row.install_path = inst.install_path.clone();
        row.executable   = inst.executable.clone();
        row.size_bytes   = inst.disk_size;
    }
    row
}

// ── Launcher ─────────────────────────────────────────────────────────────────

pub async fn launch_nile(game_id: &str) -> Result<tokio::process::Child> {
    info!("Launching Amazon game '{}' via Nile", game_id);
    Command::new("nile")
        .args(["launch", game_id])
        .spawn()
        .context("nile not found – install it from https://github.com/nicoboss/nile")
}

pub async fn launch_heroic(app_name: &str) -> Result<tokio::process::Child> {
    info!("Launching Amazon game '{}' via Heroic", app_name);
    Command::new("xdg-open")
        .arg(format!("heroic://launch/{app_name}"))
        .spawn()
        .context("xdg-open not found")
}
