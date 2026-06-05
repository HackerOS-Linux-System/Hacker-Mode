use anyhow::Result;

pub async fn run(id: &str) -> Result<()> {
    use hm_core::{db, settings::Config};
    let cfg  = Config::load_or_default(
        &dirs::config_dir().unwrap_or_default().join("hackeros/hacker-mode"),
    )?;
    let pool = db::open(&cfg.db_path()).await?;
    let game = db::games::by_id(&pool, id).await?
        .ok_or_else(|| anyhow::anyhow!("Game not found: {id}"))?;
    println!("Sending SIGTERM to processes for: {}", game.title);
    std::process::Command::new("pkill")
        .args(["-f", &game.title])
        .status()?;
    Ok(())
}
