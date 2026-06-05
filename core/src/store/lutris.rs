use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

#[derive(Debug, sqlx::FromRow)]
struct LutrisRow {
    id: i64,
    slug: String,
    name: String,
    runner: Option<String>,
    directory: Option<String>,
    installed: Option<i64>,
    lastplayed: Option<i64>,
    playtime: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct LutrisGameConfig {
    game: Option<LutrisGameSection>,
    system: Option<LutrisSystemSection>,
}

#[derive(Debug, Deserialize)]
struct LutrisGameSection {
    exe: Option<String>,
    prefix: Option<String>,
    working_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LutrisSystemSection {
    env: Option<std::collections::HashMap<String, String>>,
}

pub struct LutrisImporter {
    pga_db: PathBuf,
    config_dir: PathBuf,
    covers_dir: PathBuf,
}

impl LutrisImporter {
    pub fn new() -> Self {
        let data   = dirs::data_dir().unwrap_or_default();
        let config = dirs::config_dir().unwrap_or_default();
        Self {
            pga_db:     data.join("lutris/pga.db"),
            config_dir: config.join("lutris/games"),
            covers_dir: data.join("lutris/coverart"),
        }
    }

    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        if !self.pga_db.exists() {
            debug!("No Lutris pga.db at {}", self.pga_db.display());
            return Ok(0);
        }

        let lutris_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite://{}?mode=ro", self.pga_db.display()))
            .await
            .with_context(|| format!("Opening Lutris DB at {}", self.pga_db.display()))?;

        let rows: Vec<LutrisRow> = sqlx::query_as::<_, LutrisRow>(
            "SELECT id, slug, name, runner, directory, installed, lastplayed, playtime              FROM games WHERE installed = 1 ORDER BY lastplayed DESC",
        )
        .fetch_all(&lutris_pool)
        .await?;

        let mut count = 0;
        for lr in &rows {
            let config = self.read_game_config(&lr.slug).ok();
            let row    = lutris_to_row(lr, config.as_ref(), &self.covers_dir);
            crate::db::games::upsert(db, &row).await?;
            count += 1;
            debug!("Imported Lutris game: {} ({})", lr.name, lr.slug);
        }
        info!("Lutris import complete: {count} games");
        Ok(count)
    }

    fn read_game_config(&self, slug: &str) -> Result<LutrisGameConfig> {
        let yml = self.config_dir.join(format!("{slug}.yml"));
        let content = std::fs::read_to_string(&yml)
            .with_context(|| format!("Reading {}", yml.display()))?;
        let cfg: LutrisGameConfig = serde_yaml::from_str(&content)
            .unwrap_or(LutrisGameConfig { game: None, system: None });
        Ok(cfg)
    }
}

fn lutris_to_row(lr: &LutrisRow, config: Option<&LutrisGameConfig>, covers_dir: &PathBuf) -> GameRow {
    // Determine runner type based on Lutris runner field
    let runner_type = match lr.runner.as_deref() {
        Some("steam")            => RunnerType::SteamNative { appid: 0 }, // placeholder
        Some("wine")             => {
            let prefix = config
                .and_then(|c| c.game.as_ref())
                .and_then(|g| g.prefix.as_ref())
                .cloned()
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join(".wine")
                        .to_string_lossy()
                        .into_owned()
                });
            RunnerType::Wine { prefix: PathBuf::from(prefix) }
        }
        Some("linux") | None     => RunnerType::Native,
        Some(other)              => RunnerType::Lutris { slug: lr.slug.clone(), runner: other.to_owned() },
    };

    let mut row = GameRow::new(lr.id.to_string(), StoreSource::Lutris, &lr.name, &runner_type);
    row.install_path = lr.directory.clone();
    row.is_installed = lr.installed.unwrap_or(0) == 1;
    row.last_played  = lr.lastplayed;

    // Executable from config
    row.executable = config
        .and_then(|c| c.game.as_ref())
        .and_then(|g| g.exe.as_ref())
        .cloned();

    // Cover art from lutris coverart dir
    let cover = covers_dir.join(format!("{}.jpg", lr.slug));
    if cover.exists() {
        row.cover_path = Some(cover.to_string_lossy().into_owned());
    }
    row
}

// ── Launcher ─────────────────────────────────────────────────────────────────

pub async fn launch(slug: &str) -> Result<tokio::process::Child> {
    info!("Launching Lutris game '{slug}'");
    tokio::process::Command::new("lutris")
        .arg(format!("lutris:rungame/{slug}"))
        .spawn()
        .context("lutris not found")
}
