use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "hacker-mode", author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Mode {
    /// Uruchom jako zwykłe okno wewnątrz istniejącej sesji graficznej.
    Ui,
    /// Uruchom jako samodzielny kompozytor (sesja logowania SDDM, TTY).
    Default,
}

pub fn parse() -> Cli {
    // Domyślnie (brak argumentu) traktujemy jak `ui`, żeby dev-loop
    // (`cargo run`) był wygodny.
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        args.push("ui".to_string());
    }
    Cli::parse_from(args)
}
