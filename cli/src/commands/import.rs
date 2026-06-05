use anyhow::Result;

pub async fn run(source: Option<&str>) -> Result<()> {
    use hm_core::{db, settings::Config, store::*};

    let cfg  = Config::load_or_default(
        &dirs::config_dir().unwrap_or_default().join("hackeros/hacker-mode"),
    )?;
    let pool = db::open(&cfg.db_path()).await?;
    let mut total = 0usize;

    let do_all    = source.is_none();
    let do_source = |s: &str| source.map(|f| f == s).unwrap_or(true);

    if do_source("steam") {
        if let Some(imp) = steam::SteamImporter::new() {
            let n = imp.import_all(&pool).await?;
            println!("Steam: {n} games");
            total += n;
        } else {
            println!("Steam: not found");
        }
    }
    if do_source("epic") {
        let n = epic::EpicImporter::new().import_all(&pool).await?;
        println!("Epic:  {n} games");
        total += n;
    }
    if do_source("gog") {
        let n = gog::GogImporter::new().import_all(&pool).await?;
        println!("GOG:   {n} games");
        total += n;
    }
    if do_source("amazon") {
        let n = amazon::AmazonImporter::new().import_all(&pool).await?;
        println!("Amazon: {n} games");
        total += n;
    }
    if do_source("itchio") {
        let n = itchio::ItchioImporter::new().import_all(&pool).await?;
        println!("itch.io: {n} games");
        total += n;
    }
    if do_source("lutris") {
        let n = lutris::LutrisImporter::new().import_all(&pool).await?;
        println!("Lutris: {n} games");
        total += n;
    }

    println!("─────────────────");
    println!("Total: {total} games imported");
    Ok(())
}
