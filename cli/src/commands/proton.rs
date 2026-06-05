use anyhow::Result;

pub async fn run(install_ge: bool) -> Result<()> {
    if install_ge {
        println!("Installing latest GE-Proton…");
        let v = hm_core::runner::proton::install_ge_proton_latest().await?;
        println!("Installed: {}", v.name);
    } else {
        let versions = hm_core::runner::proton::find_proton_versions();
        if versions.is_empty() {
            println!("No Proton versions found.");
            println!("Run: hm proton --install-ge");
        } else {
            for v in versions {
                println!("  {}", v.name);
            }
        }
    }
    Ok(())
}
