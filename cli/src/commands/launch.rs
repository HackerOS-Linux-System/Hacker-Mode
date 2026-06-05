use anyhow::Result;
use tracing::info;

pub async fn run(id: &str, proton: Option<&str>) -> Result<()> {
    use hm_core::{db, runner::{GameRunner, RunnerType}, settings::Config};

    let cfg = Config::load_or_default(
        &dirs::config_dir().unwrap_or_default().join("hackeros/hacker-mode"),
    )?;
    let pool = db::open(&cfg.db_path()).await?;

    let game = db::games::by_id(&pool, id).await?
        .ok_or_else(|| anyhow::anyhow!("Game not found: {id}"))?;

    println!("Launching: {} [{}]", game.title, game.source);

    let runner = GameRunner::new(pool.clone());

    // Override Proton if requested
    let mut game2 = game.clone();
    if let Some(version) = proton {
        game2.runner = serde_json::to_string(&RunnerType::Proton { version: version.to_owned() })?;
    }

    let running = runner.launch(&game2).await?;
    println!("PID: {}", running.pid);

    GameRunner::monitor(pool, running).await;
    Ok(())
}
