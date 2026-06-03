pub mod db;
pub mod store;
pub mod runner;
pub mod auth;
pub mod imports;
pub mod settings;
pub mod perf;
pub mod ipc;

pub use db::Database;

use anyhow::Result;

/// Initialise the entire core subsystem.
/// Call once from the UI process before creating any other handles.
pub async fn init(config_dir: std::path::PathBuf) -> Result<crate::settings::Config> {
    tracing::info!("hm-core init, config_dir={}", config_dir.display());
    let cfg = settings::Config::load_or_default(&config_dir)?;
    db::run_migrations(&cfg.db_path()).await?;
    Ok(cfg)
}
