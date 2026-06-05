use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, info};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

pub struct ItchioImporter {
    butler_db: PathBuf,
}

impl ItchioImporter {
    pub fn new() -> Self {
        let config = dirs::config_dir().unwrap_or_default();
        Self { butler_db: config.join("itch/db/butler.db") }
    }

    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        if !self.butler_db.exists() {
            debug!("No itch.io butler DB at {}", self.butler_db.display());
            return Ok(0);
        }

        let url = format!("sqlite://{}?mode=ro", self.butler_db.display());
        let butler_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .with_context(|| format!("Opening butler DB at {}", self.butler_db.display()))?;

        // Query games and caves in a single flexible query
        let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>)> =
            sqlx::query_as(
                r#"SELECT c.id, g.title, g.cover_url,
                          l.path || '/' || c.install_folder_name as install_path,
                          c.id as cave_id
                   FROM caves c
                   JOIN games g ON g.id = c.game_id
                   LEFT JOIN install_locations l ON l.id = c.install_location_id"#
            )
            .fetch_all(&butler_pool)
            .await
            .unwrap_or_default();

        let mut count = 0;
        for (cave_id, title, cover_url, install_path, _) in &rows {
            let is_installed = install_path.as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false);

            let mut row = GameRow::new(
                cave_id,
                StoreSource::Itchio,
                title,
                &RunnerType::Itchio { cave_id: cave_id.clone() },
            );
            row.cover_path   = cover_url.clone();
            row.install_path = install_path.clone();
            row.is_installed = is_installed;

            crate::db::games::upsert(db, &row).await?;
            count += 1;
        }

        info!("itch.io import complete: {count} games");
        Ok(count)
    }
}

pub async fn launch_cave(cave_id: &str) -> Result<tokio::process::Child> {
    info!("Launching itch.io cave '{cave_id}'");
    tokio::process::Command::new("xdg-open")
        .arg(format!("itch://caves/{cave_id}/launch"))
        .spawn()
        .context("xdg-open not found")
}
