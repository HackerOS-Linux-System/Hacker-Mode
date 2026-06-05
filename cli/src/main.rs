mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name        = "hm",
    about       = "HackerOS Hacker Mode CLI",
    version     = env!("CARGO_PKG_VERSION"),
    author      = "HackerOS Team",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch a game by ID
    Launch {
        /// Game ID (UUID from the library)
        id: String,
        /// Proton version override (e.g. "GE-Proton9-20")
        #[arg(short, long)]
        proton: Option<String>,
    },
    /// Import games from all configured sources
    Import {
        /// Only import from a specific source (steam|epic|gog|amazon|itchio|lutris)
        #[arg(short, long)]
        source: Option<String>,
    },
    /// List games in the library
    List {
        /// Filter by source
        #[arg(short, long)]
        source: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show currently running games
    Running,
    /// Kill a running game
    Kill {
        id: String,
    },
    /// List installed Proton versions
    Proton {
        /// Install latest GE-Proton
        #[arg(long)]
        install_ge: bool,
    },
    /// Network management
    Network {
        #[command(subcommand)]
        cmd: commands::network::NetworkCmd,
    },
    /// Send a power action
    Power {
        /// Action: shutdown|reboot|suspend|hibernate|logout|restart_hm
        action: String,
    },
    /// Show performance snapshot
    Perf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("warn"))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Launch { id, proton } => {
            commands::launch::run(&id, proton.as_deref()).await
        }
        Commands::Import { source } => {
            commands::import::run(source.as_deref()).await
        }
        Commands::List { source, json } => {
            commands::list::run(source.as_deref(), json).await
        }
        Commands::Running => {
            commands::running::run().await
        }
        Commands::Kill { id } => {
            commands::kill::run(&id).await
        }
        Commands::Proton { install_ge } => {
            commands::proton::run(install_ge).await
        }
        Commands::Network { cmd } => {
            commands::network::run(cmd).await
        }
        Commands::Power { action } => {
            commands::power::run(&action).await
        }
        Commands::Perf => {
            commands::perf::run().await
        }
    }
}
