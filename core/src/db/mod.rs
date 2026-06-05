pub mod games;
pub mod playtime;

pub use games::GameRow;
pub use playtime::PlaytimeRow;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;

pub type Database = SqlitePool;

pub async fn open(db_path: &Path) -> Result<Database> {
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(db_path: &Path) -> Result<()> {
    let pool = open(db_path).await?;
    // Apply schema directly since we avoid compile-time migration macros
    sqlx::query(include_str!("../../migrations/0001_initial.sql"))
        .execute(&pool)
        .await
        .ok(); // Ignore errors (table already exists)
    tracing::info!("DB schema applied");
    Ok(())
}
