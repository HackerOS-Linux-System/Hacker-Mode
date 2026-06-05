pub mod proton;
pub mod process;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::db::{games::GameRow, playtime, Database};

// ── RunnerType ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunnerType {
    Native,
    SteamNative   { appid: u32 },
    Proton        { version: String },
    Wine          { prefix: PathBuf },
    Heroic        { app_name: String },
    Lutris        { slug: String, runner: String },
    Itchio        { cave_id: String },
    Nile          { id: String },
}

// ── Running game ──────────────────────────────────────────────────────────────

pub struct RunningGame {
    pub game_id:       String,
    pub pid:           u32,
    pub session_start: i64,
    child: Arc<Mutex<tokio::process::Child>>,
}

impl RunningGame {
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        let mut child = self.child.lock().await;
        Ok(child.wait().await?)
    }

    pub async fn kill(&mut self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await.ok();
        Ok(())
    }
}

// ── GameRunner ────────────────────────────────────────────────────────────────

pub struct GameRunner {
    pub db:    Database,
    pub steam: Option<crate::store::steam::SteamInstall>,
}

impl GameRunner {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            steam: crate::store::steam::SteamInstall::detect(),
        }
    }

    pub async fn launch(&self, game: &GameRow) -> Result<RunningGame> {
        let runner = game.runner_type().unwrap_or(RunnerType::Native);
        info!("Launching '{}' with runner {:?}", game.title, runner);

        let child = match &runner {
            RunnerType::Native => {
                let exe = game.executable.as_deref()
                    .or(game.install_path.as_deref())
                    .context("No executable for native game")?;
                let mut cmd = tokio::process::Command::new(exe);
                if let Some(ref dir) = game.install_path {
                    cmd.current_dir(dir);
                }
                cmd.spawn()?
            }

            RunnerType::SteamNative { appid } => {
                let steam = self.steam.as_ref().context("Steam not found")?;
                crate::store::steam::launch(steam, *appid).await?
            }

            RunnerType::Proton { version } => {
                let exe = game.executable.as_deref().context("No executable")?;
                let prefix = game.wine_prefix.as_deref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        dirs::data_dir().unwrap_or_default()
                            .join(format!("hacker-mode/prefixes/{}", game.id))
                    });
                proton::launch(exe, version, &prefix, game.install_path.as_deref()).await?
            }

            RunnerType::Wine { prefix } => {
                let exe = game.executable.as_deref().context("No executable")?;
                proton::launch_wine(exe, prefix, game.install_path.as_deref()).await?
            }

            RunnerType::Heroic { app_name } => {
                match crate::store::epic::launch_heroic(app_name).await {
                    Ok(c)  => c,
                    Err(e) => {
                        warn!("Heroic launch failed ({e}), trying Legendary");
                        crate::store::epic::launch_legendary(app_name).await?
                    }
                }
            }

            RunnerType::Lutris { slug, .. } => {
                crate::store::lutris::launch(slug).await?
            }

            RunnerType::Itchio { cave_id } => {
                crate::store::itchio::launch_cave(cave_id).await?
            }

            RunnerType::Nile { id } => {
                crate::store::amazon::launch_nile(id).await?
            }
        };

        let pid           = child.id().unwrap_or(0);
        let session_start = playtime::start_session(&self.db, &game.id).await?;
        crate::db::games::set_last_played(&self.db, &game.id).await?;

        Ok(RunningGame {
            game_id: game.id.clone(),
            pid,
            session_start,
            child: Arc::new(Mutex::new(child)),
        })
    }

    pub async fn monitor(db: Database, mut running: RunningGame) {
        let game_id = running.game_id.clone();
        let start   = running.session_start;
        tokio::spawn(async move {
            let _ = running.wait().await;
            if let Err(e) = playtime::end_session(&db, &game_id, start).await {
                warn!("Failed to record playtime: {e}");
            } else {
                info!("Session ended for {game_id}");
            }
        });
    }
}
