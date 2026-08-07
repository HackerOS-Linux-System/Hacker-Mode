use super::{tool_available, Game, Platform, StoreError, StoreProvider};
use serde_json::Value;
use std::process::Command;

#[derive(Default)]
pub struct LutrisProvider;

impl StoreProvider for LutrisProvider {
    fn platform(&self) -> Platform {
        Platform::Lutris
    }

    fn is_available(&self) -> bool {
        tool_available("lutris")
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("lutris")
            .args(["-l", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("lutris".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::ExecFailed(
                "lutris".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("lutris -l --json".into(), e.to_string()))?;

        let entries = parsed.as_array().cloned().unwrap_or_default();
        let games = entries
            .into_iter()
            .filter_map(|entry| {
                let slug = entry.get("slug")?.as_str()?.to_string();
                let title = entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&slug)
                    .to_string();
                let directory = entry
                    .get("directory")
                    .and_then(Value::as_str)
                    .map(String::from);

                Some(Game {
                    id: slug,
                    title,
                    platform: Platform::Lutris,
                    installed: true,
                    install_dir: directory,
                    cover_path: None,
                    playtime_minutes: None,
                })
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // Lutris obsługuje protokół `lutris:` do wyzwalania własnych
        // instalatorów gier (pobranych z lutris.net) — jeśli gra o danym
        // slugu ma zdefiniowany instalator, otworzy się kreator instalacji
        // w oknie Lutrisa.
        let mut cmd = Command::new("lutris");
        cmd.arg(format!("lutris:{game_id}"));
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("lutris");
        cmd.arg(format!("rungame/{game_id}"));
        Ok(cmd)
    }
}
