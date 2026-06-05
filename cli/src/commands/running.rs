use anyhow::Result;

pub async fn run() -> Result<()> {
    // Without an IPC client here we check /proc for hm processes
    println!("Running game processes:");
    let output = std::process::Command::new("pgrep")
        .args(["-a", "-f", "proton|wine|legendary|lutris"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        println!("  (none)");
    } else {
        println!("{stdout}");
    }
    Ok(())
}
