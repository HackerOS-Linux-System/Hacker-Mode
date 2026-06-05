use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use super::RunningGame;

/// Global registry of active game processes.
pub struct ProcessRegistry {
    games: Arc<RwLock<HashMap<String, Arc<Mutex<RunningGame>>>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self { games: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn register(&self, game: RunningGame) {
        let game_id = game.game_id.clone();
        let pid     = game.pid;
        self.games.write().await.insert(game_id.clone(), Arc::new(Mutex::new(game)));
        info!("Registered game process: {game_id} (PID {pid})");
    }

    pub async fn is_running(&self, game_id: &str) -> bool {
        self.games.read().await.contains_key(game_id)
    }

    pub async fn running_ids(&self) -> Vec<String> {
        self.games.read().await.keys().cloned().collect()
    }

    pub async fn kill(&self, game_id: &str) -> bool {
        if let Some(handle) = self.games.write().await.remove(game_id) {
            let mut rg = handle.lock().await;
            if let Err(e) = rg.kill().await {
                warn!("Kill failed for {game_id}: {e}");
            }
            info!("Killed game process: {game_id}");
            true
        } else {
            false
        }
    }

    pub async fn remove_exited(&self) {
        // Called periodically to clean up exited processes
        let ids: Vec<String> = self.games.read().await.keys().cloned().collect();
        for id in ids {
            let games = self.games.read().await;
            if let Some(handle) = games.get(&id) {
                let rg = handle.lock().await;
                // Check if process is still alive via /proc/<pid>
                if rg.pid > 0 && !std::path::Path::new(&format!("/proc/{}", rg.pid)).exists() {
                    drop(rg); drop(games);
                    self.games.write().await.remove(&id);
                    info!("Removed exited game: {id}");
                }
            }
        }
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self { Self::new() }
}
