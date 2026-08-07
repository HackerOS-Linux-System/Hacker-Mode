use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("ping") => hacker_mode_ipc::call(hacker_mode_ipc::IpcCall::Ping, Duration::from_secs(2)).map(|_| ()).map_err(anyhow::Error::from),
        Some("launch") if args.len() >= 2 => {
            hacker_mode_ipc::launch_shell(&args[1], &args[2..]).map_err(anyhow::Error::from)
        }
        Some("shutdown") => hacker_mode_ipc::shutdown_compositor(Duration::from_secs(2)).map_err(anyhow::Error::from),
        _ => {
            eprintln!("użycie: hacker-mode-ipcctl <ping|shutdown|launch <cmd> [args...]>");
            std::process::exit(2);
        }
    };

    if let Err(err) = result {
        eprintln!("hacker-mode-ipcctl: {err}");
        std::process::exit(1);
    }
}
