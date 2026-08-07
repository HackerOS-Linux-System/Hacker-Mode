use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::stores::{self, Platform};
use crate::state::AppState;

const LAUNCH_COOLDOWN: Duration = Duration::from_secs(10);

/// Jak długo czekamy, aż okno nowo uruchomionego procesu (gry/launchera)
/// pojawi się w `ListWindows` comphwde, żeby je zmaksymalizować - patrz
/// [`hacker_mode_ipc::maximize_next_new_window`].
const NEW_WINDOW_TIMEOUT: Duration = Duration::from_secs(15);
const NEW_WINDOW_POLL_INTERVAL: Duration = Duration::from_millis(300);
const IPC_CALL_TIMEOUT: Duration = Duration::from_secs(2);

/// Uruchamia grę z danej platformy po jej identyfikatorze (patrz
/// `stores::Game::id`).
pub fn launch_game(app: AppHandle, state: State<AppState>, platform: Platform, game_id: String) -> Result<(), String> {
    let cmd = stores::build_launch_command(platform, &game_id).map_err(|e| e.to_string())?;
    run_wrapped(app, state, cmd, format!("{}:{}", platform.label(), game_id))
}

/// Uruchamia bezpośrednio klienta GUI danego sklepu/launchera (np. otwarcie
/// pełnego Steama zamiast konkretnej gry) — odpowiednik przycisków ze
/// starego widoku "Hacker Menu".
pub fn launch_store_client(app: AppHandle, state: State<AppState>, name: &str) -> Result<(), String> {
    let cmd = match name {
        "steam" => steam_client_command(),
        "heroic" => flatpak_command("com.heroicgameslauncher.hgl"),
        "hyperplay" => flatpak_command("xyz.hyperplay.HyperPlay"),
        "lutris" => native_command("lutris"),
        other => return Err(format!("Nieznany klient sklepu: {other}")),
    };
    run_wrapped(app, state, cmd, name.to_string())
}

/// Instaluje grę (bez trybu wrapper — instalacja nie chowa powłoki, tylko
/// pokazuje postęp/logi w UI poprzez event `hacker-mode://install-progress`
/// i `hacker-mode://install-finished`).
pub fn install_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    let mut cmd = stores::build_install_command(platform, &game_id).map_err(|e| e.to_string())?;
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić instalacji: {e}"))?;

    let label = format!("{}:{}", platform.label(), game_id);

    // Przekierowanie stdout instalatora do frontendu na żywo (paski
    // postępu `legendary`/`gogdl`/`nile` piszą procent ukończenia na
    // stdout) — proste, liniowe strumieniowanie, bez parsowania procentów
    // (to zależy od narzędzia; frontend może same linie wyświetlić jako
    // log, ewentualnie sparsować wzorce typu "NN.N%" po swojej stronie).
    if let Some(stdout) = child.stdout.take() {
        let app_stdout = app.clone();
        let label_stdout = label.clone();
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            // Dopasowuje wzorce w stylu "42%", "42.7%", "[42/100]" — różne
            // narzędzia (`legendary`, `gogdl`, `nile`) formatują pasek
            // postępu inaczej, więc próbujemy kilku wariantów zamiast
            // zakładać jeden konkretny format.
            let percent_re = regex::Regex::new(r"(\d{1,3}(?:\.\d+)?)\s*%").unwrap();
            for line in BufReader::new(stdout).lines().flatten() {
                let percent = percent_re
                    .captures(&line)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .filter(|p| (0.0..=100.0).contains(p));
                let _ = app_stdout.emit(
                    "hacker-mode://install-progress",
                    serde_json::json!({ "label": label_stdout, "line": line, "percent": percent }),
                );
            }
        });
    }

    let app_for_wait = app.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        tracing::info!(label = %label, ok, "Instalacja zakończona");
        let _ = app_for_wait.emit(
            "hacker-mode://install-finished",
            serde_json::json!({ "label": label, "ok": ok }),
        );
    });

    Ok(())
}

pub fn uninstall_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    let mut cmd = stores::build_uninstall_command(platform, &game_id).map_err(|e| e.to_string())?;
    let label = format!("{}:{}", platform.label(), game_id);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić odinstalowania: {e}"))?;
    std::thread::spawn(move || {
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        tracing::info!(label = %label, ok, "Odinstalowanie zakończone");
        let _ = app.emit(
            "hacker-mode://uninstall-finished",
            serde_json::json!({ "label": label, "ok": ok }),
        );
    });
    Ok(())
}

/// Uruchamia proces logowania do sklepu — zwraca `LoginFlow` z URL-em do
/// pokazania w osadzonym webview.
pub fn login_start(platform: Platform) -> Result<stores::LoginFlow, String> {
    stores::login_start(platform).map_err(|e| e.to_string())
}

pub fn login_submit(platform: Platform, code: String) -> Result<(), String> {
    stores::login_submit(platform, &code).map_err(|e| e.to_string())
}

pub fn is_logged_in(platform: Platform) -> bool {
    stores::is_logged_in(platform)
}

fn steam_client_command() -> Command {
    if which::which("steam").is_ok() {
        Command::new("steam")
    } else {
        flatpak_command("com.valvesoftware.Steam")
    }
}

fn flatpak_command(app_id: &str) -> Command {
    let mut cmd = Command::new("flatpak");
    cmd.args(["run", app_id]);
    cmd
}

fn native_command(bin: &str) -> Command {
    Command::new(bin)
}

fn run_wrapped(app: AppHandle, state: State<AppState>, mut cmd: Command, label: String) -> Result<(), String> {
    // Debounce: unikamy przypadkowego wielokrotnego odpalenia tej samej
    // aplikacji przy podwójnym kliknięciu / powtórzonym evencie z pada.
    static LAST_LAUNCH: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Instant>>> =
        std::sync::OnceLock::new();
    let map = LAST_LAUNCH.get_or_init(|| std::sync::Mutex::new(Default::default()));
    {
        let mut guard = map.lock().unwrap();
        if let Some(last) = guard.get(&label) {
            if last.elapsed() < LAUNCH_COOLDOWN {
                return Err(format!(
                    "Poczekaj chwilę przed ponownym uruchomieniem „{label}”."
                ));
            }
        }
        guard.insert(label.clone(), Instant::now());
    }

    let wrapper_enabled = state.settings.lock().unwrap().wrapper_mode_enabled;
    let is_dev = state.is_dev();

    // Migawka okien SPRZED odpalenia procesu — comphwde nie ma
    // dedykowanego "WrapperModeEnter/Exit" jak dawny własny kompozytor;
    // zamiast tego chowamy własne okno powłoki i po prostu wypatrujemy,
    // które okno jest NOWE względem tej migawki, żeby je zmaksymalizować.
    // Patrz `hacker_mode_ipc::maximize_next_new_window`.
    let known_windows = if wrapper_enabled && !is_dev {
        hacker_mode_ipc::known_window_ids(IPC_CALL_TIMEOUT).unwrap_or_default()
    } else {
        Vec::new()
    };

    if wrapper_enabled && !is_dev {
        enter_wrapper_mode(&app, &state);
    }

    cmd.env("XDG_SESSION_TYPE", "wayland");
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić „{label}”: {e}"))?;

    let app_for_thread = app.clone();
    let wrapper_enabled_for_thread = wrapper_enabled && !is_dev;

    if wrapper_enabled_for_thread {
        // Osobny wątek: czeka aż nowe okno gry/launchera pojawi się w
        // comphwde i maksymalizuje je. Nie blokuje ani UI, ani wątku
        // czekającego na zakończenie procesu poniżej.
        std::thread::spawn(move || {
            match hacker_mode_ipc::maximize_next_new_window(&known_windows, NEW_WINDOW_TIMEOUT, NEW_WINDOW_POLL_INTERVAL) {
                Ok(id) => tracing::info!(window_id = id, "Nowe okno zmaksymalizowane (tryb wrapper)"),
                Err(e) => tracing::debug!(error = %e, "Nie doczekano się nowego okna do zmaksymalizowania"),
            }
        });
    }

    std::thread::spawn(move || {
        let status = child.wait();
        tracing::info!(label = %label, ?status, "Proces zakończony");

        if wrapper_enabled_for_thread {
            exit_wrapper_mode(&app_for_thread);
        }

        let _ = app_for_thread.emit(
            "hacker-mode://app-closed",
            serde_json::json!({ "label": label }),
        );
    });

    Ok(())
}

fn enter_wrapper_mode(app: &AppHandle, state: &State<AppState>) {
    state.wrapper_active.store(true, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("shell") {
        let _ = win.hide();
    }
    // Chowamy też po stronie comphwde (nie tylko po stronie Tauri) —
    // patrz `enter_wrapper`'s doc comment: to `MinimizeWindow` na naszym
    // własnym oknie znalezionym po `SHELL_APP_ID`, ten sam prymityw,
    // którego comphwde już używa dla SDE.
    if let Err(e) = hacker_mode_ipc::enter_wrapper(IPC_CALL_TIMEOUT) {
        tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
    }
}

fn exit_wrapper_mode(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.wrapper_active.store(false, Ordering::Relaxed);
    }
    if let Some(win) = app.get_webview_window("shell") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    if let Err(e) = hacker_mode_ipc::exit_wrapper(IPC_CALL_TIMEOUT) {
        tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
    }
}

/// Zamyka wszystkie znane procesy launcherów/sklepów (odpowiednik
/// `restartApps` z wersji 0.1) i przywraca powłokę.
pub fn restart_apps(app: &AppHandle) -> ActionSummary {
    let targets = ["steam", "heroic", "hyperplay", "lutris", "legendary", "gogdl", "nile"];
    let mut killed = Vec::new();
    for target in targets {
        if Command::new("pkill").args(["-f", target]).status().is_ok() {
            killed.push(target.to_string());
        }
    }
    exit_wrapper_mode(app);
    ActionSummary { killed }
}

#[derive(serde::Serialize)]
pub struct ActionSummary {
    pub killed: Vec<String>,
}
