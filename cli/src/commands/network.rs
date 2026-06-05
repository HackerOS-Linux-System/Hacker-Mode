use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum NetworkCmd {
    /// List Wi-Fi networks
    Wifi,
    /// Connect to Wi-Fi
    Connect { ssid: String, #[arg(short)] password: Option<String> },
    /// Show active connection
    Status,
}

pub async fn run(cmd: NetworkCmd) -> Result<()> {
    use hm_core::settings::network;
    match cmd {
        NetworkCmd::Wifi => {
            let nets = network::list_wifi().await?;
            for n in nets {
                println!("{} {}% {}", if n.in_use { "*" } else { " " }, n.signal, n.ssid);
            }
        }
        NetworkCmd::Connect { ssid, password } => {
            network::connect_wifi(&ssid, password.as_deref()).await?;
            println!("Connected to {ssid}");
        }
        NetworkCmd::Status => {
            match network::active_connection().await {
                Some(c) => println!("{} ({})", c.name, c.state),
                None    => println!("No active connection"),
            }
        }
    }
    Ok(())
}
