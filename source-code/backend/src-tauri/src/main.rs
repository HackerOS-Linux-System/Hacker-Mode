#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use hacker_mode_lib::{commands, state::AppState};
use tauri::Manager;

/// `hacker-mode` — jedna binarka: powłoka do gier (Tauri + Solid.js),
/// uruchamiana przez `comphwde --extern-hacker-mode` (projekt HWDE) jako
/// jego klient Wayland — patrz `scripts/hacker-mode-session`.
///
/// Komendy:
///   hacker-mode        uruchamia normalnie — zakłada, że proces został
///                       wystartowany przez `comphwde` przez `LaunchApp`
///                       (patrz `scripts/hacker-mode-session`), z sesji z
///                       menedżera logowania albo bezpośrednio z TTY.
///   hacker-mode dev     uruchamia samą powłokę Tauri w oknie, do pracy nad
///                       UI — większość integracji systemowych/comphwde
///                       jest wyłączona lub bezpiecznie no-op.
#[derive(Parser, Debug)]
#[command(name = "hacker-mode", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Mode>,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Tryb deweloperski: okno (nie fullscreen), bez integracji z
    /// comphwde/zasilaniem, przydatny do iteracji nad UI.
    Dev,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let dev_mode = matches!(cli.command, Some(Mode::Dev));

    init_logging()?;
    tracing::info!(dev_mode, "Uruchamianie Hacker Mode");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Drugie uruchomienie tylko przywraca istniejące okno zamiast
            // odpalać kolejną instancję powłoki.
            if let Some(win) = app.get_webview_window("shell") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .manage(AppState::new(dev_mode))
        .setup(move |app| {
            if let Some(win) = app.get_webview_window("shell") {
                if dev_mode {
                    // W trybie dev pracujemy w oknie, nie na pełnym ekranie —
                    // wygodniej debugować i nie "porywa" to całego pulpitu
                    // podczas developmentu.
                    let _ = win.set_fullscreen(false);
                    let _ = win.set_decorations(true);
                    let _ = win.set_title("Hacker Mode (dev)");
                } else {
                    // Powłoka jest teraz klientem comphwde
                    // (`--extern-hacker-mode`), a nie jego rodzicem — gdy
                    // powłoka się zamyka, kończy się cała sesja. Poproś
                    // comphwde o zamknięcie, żeby
                    // `scripts/hacker-mode-session` (które czeka na proces
                    // kompozytora) zakończyło się od razu, zamiast wisieć
                    // aż coś inne (np. menedżer logowania) zabije
                    // kompozytora osobno.
                    let win_for_close = win.clone();
                    win_for_close.on_window_event(move |event| {
                        if matches!(event, tauri::WindowEvent::Destroyed) {
                            if let Err(e) = hacker_mode_ipc::shutdown_compositor(std::time::Duration::from_secs(2)) {
                                tracing::debug!(error = %e, "Nie udało się poprosić comphwde o zamknięcie (być może już nie działa)");
                            }
                        }
                    });
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_games,
            commands::launch_game,
            commands::launch_store_client,
            commands::restart_apps,
            commands::install_game,
            commands::uninstall_game,
            commands::fetch_game_details,
            commands::search_steam_store,
            commands::open_steam_store_page,
            commands::store_is_logged_in,
            commands::store_login_start,
            commands::store_login_submit,
            commands::open_store_login_window,
            commands::submit_store_login_code,
            commands::get_settings,
            commands::save_settings,
            commands::is_dev_mode,
            commands::audio_action,
            commands::display_action,
            commands::network_action,
            commands::bluetooth_action,
            commands::power_action,
            commands::switch_to_desktop_session,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Błąd uruchomienia Tauri: {e}"))?;

    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    let log_dir = dirs::state_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("hacker-mode");
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "hacker-mode.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // `guard` musi żyć przez cały czas trwania programu, żeby bufor
    // logów był flushowany — "przeciekamy" go celowo (typowy wzorzec
    // z `tracing-appender`).
    Box::leak(Box::new(guard));

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    Ok(())
}
