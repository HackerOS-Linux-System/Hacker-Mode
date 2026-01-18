#![allow(clippy::uninlined_format_args)]

mod cursor;
mod drawing;
mod focus;
mod render;
mod shell;
mod state;
mod udev;
mod winit;
mod x11;

fn main() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();
    } else {
        tracing_subscriber::fmt().compact().init();
    }

    profiling::register_thread!("Main Thread");

    let arg = ::std::env::args().nth(1);
    match arg.as_ref().map(|s| &s[..]) {
        Some("--winit") => {
            tracing::info!("Starting hacker-compositor with winit backend (nested for games)");
            crate::winit::run_winit();
        }
        Some("--tty-udev") => {
            tracing::info!("Starting hacker-compositor on a tty using udev");
            crate::udev::run_udev();
        }
        Some("--x11") => {
            tracing::info!("Starting hacker-compositor with x11 backend");
            crate::x11::run_x11();
        }
        Some(other) => {
            tracing::error!("Unknown backend: {}", other);
        }
        None => {
            println!("USAGE: hacker-compositor --backend");
            println!();
            println!("Possible backends are:");
            println!("\t--winit (recommended for nested in Hacker Mode)");
            println!("\t--tty-udev");
            println!("\t--x11");
        }
    }
}

