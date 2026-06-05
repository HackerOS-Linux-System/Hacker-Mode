use anyhow::Result;

pub async fn run(source: Option<&str>, json: bool) -> Result<()> {
    use hm_core::{db, settings::Config};

    let cfg  = Config::load_or_default(
        &dirs::config_dir().unwrap_or_default().join("hackeros/hacker-mode"),
    )?;
    let pool = db::open(&cfg.db_path()).await?;

    let games = if let Some(src) = source {
        use std::str::FromStr;
        let s = hm_core::store::StoreSource::from_str(src)?;
        db::games::by_source(&pool, s).await?
    } else {
        db::games::all(&pool).await?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&games)?);
    } else {
        println!("{:<40} {:<10} {:<12}", "TITLE", "SOURCE", "INSTALLED");
        println!("{}", "─".repeat(65));
        for g in &games {
            println!("{:<40} {:<10} {:<12}",
                if g.title.len() > 38 { format!("{}…", &g.title[..37]) } else { g.title.clone() },
                g.source,
                if g.is_installed { "yes" } else { "no" }
            );
        }
        println!("\n{} games", games.len());
    }
    Ok(())
}
