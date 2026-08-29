use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

/// `~/.hackeros/Hacker-Mode/` — wspólny katalog danych Hacker Mode w
/// ramach szerszego systemu HackerOS (stąd `.hackeros` w nazwie, nie
/// `.config`/`.local/share` — to samo miejsce, którego (przyszłościowo)
/// mogą używać też inne komponenty HackerOS, nie tylko Hacker Mode).
fn hackeros_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".hackeros")
        .join("Hacker-Mode")
}

fn marker_path() -> PathBuf {
    hackeros_dir().join(".no-first-time")
}

/// `~/.hackeros/Hacker-Mode/env/` — wirtualne środowisko Pythona, w którym
/// ląduje `legendary`/`gogdl`/`nile` zamiast systemowego `pip install`.
pub fn venv_dir() -> PathBuf {
    hackeros_dir().join("env")
}

/// Katalog `bin/` wewnątrz venv — tu `pip install` stawia wykonywalne
/// skrypty (`legendary`, `gogdl`, `nile`) po instalacji.
pub fn venv_bin_dir() -> PathBuf {
    venv_dir().join("bin")
}

/// Dopisuje `venv_bin_dir()` na POCZĄTEK zmiennej `PATH` procesu Hacker
/// Mode, o ile taki katalog istnieje — wywoływane raz, na starcie
/// aplikacji (patrz `main.rs`). Dzięki temu KAŻDE dotychczasowe
/// `Command::new("legendary")`/`which::which("legendary")` w całym
/// kodzie (patrz `commands/stores/{epic,gog,amazon}.rs`) automatycznie
/// znajdzie narzędzie zainstalowane do venv, bez potrzeby zmiany
/// pojedynczego z kilkunastu miejsc, które te narzędzia wywołują —
/// wystarczy, że venv-owy `bin/` ma pierwszeństwo przed resztą `PATH`.
///
/// `unsafe`: `std::env::set_var` jest `unsafe fn` od Rust 1.82 (soundness
/// fix dot. wielowątkowego dostępu do env) — na starszych toolchainach
/// (jak używany w tym repo do lokalnej weryfikacji, Rust 1.75) ta sama
/// funkcja jest bezpieczna, więc blok `unsafe` jest tam technicznie
/// zbędny, ale nieszkodliwy (`#[allow(unused_unsafe)]` tłumi wynikający z
/// tego warning) — dzięki temu ten kod kompiluje się identycznie na obu.
/// Wołane raz, na starcie, zanim jakikolwiek inny wątek mógłby czytać/
/// pisać `PATH`, więc nie ma tu realnego ryzyka data race, mimo że
/// kompilator nowszych wersji Rusta nie jest w stanie tego sam zweryfikować.
#[allow(unused_unsafe)]
pub fn prepend_venv_to_path() {
    let bin = venv_bin_dir();
    if !bin.is_dir() {
        return;
    }
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::env::split_paths(&current).collect();
    if paths.first() == Some(&bin) {
        return; // już dopisane (np. ponowna inicjalizacja w testach)
    }
    paths.insert(0, bin);
    if let Ok(joined) = std::env::join_paths(paths) {
        unsafe {
            std::env::set_var("PATH", joined);
        }
    }
}

#[tauri::command]
pub fn is_first_run() -> bool {
    !marker_path().is_file()
}

#[tauri::command]
pub fn mark_first_run_complete() -> Result<(), String> {
    let dir = hackeros_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Nie udało się utworzyć {}: {e}", dir.display()))?;
    std::fs::write(marker_path(), b"").map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherToolStatus {
    pub id: &'static str,
    pub label: &'static str,
    /// Czy narzędzie jest już dostępne — w systemowym `PATH` ALBO w
    /// venv Hacker Mode (patrz `prepend_venv_to_path`, wywołane już przy
    /// starcie aplikacji, więc `which::which` tu też to uwzględnia).
    pub available: bool,
}

#[tauri::command]
pub fn first_run_tool_status() -> Vec<LauncherToolStatus> {
    LAUNCHER_TOOLS
        .iter()
        .map(|t| LauncherToolStatus {
            id: t.id,
            label: t.label,
            available: which::which(t.id).is_ok(),
        })
        .collect()
}

struct LauncherTool {
    id: &'static str,
    label: &'static str,
    /// Argument dla `pip install` — nazwa pakietu PyPI albo `git+https://…`.
    pip_target: &'static str,
}

/// Patrz komentarz na początku pliku po źródła/uzasadnienie każdego z tych
/// trzech `pip_target`. Świadomie NIE ma tu Steama (patrz moduł-doc) ani
/// Lutrisa/Wine (to system'owe pakiety dystrybucji, nie coś do `pip`).
const LAUNCHER_TOOLS: &[LauncherTool] = &[
    LauncherTool { id: "legendary", label: "Legendary (Epic Games)", pip_target: "legendary-gl" },
    LauncherTool {
        id: "gogdl",
        label: "gogdl (GOG)",
        pip_target: "git+https://github.com/Heroic-Games-Launcher/heroic-gogdl",
    },
    LauncherTool { id: "nile", label: "nile (Amazon Games)", pip_target: "git+https://github.com/imLinguin/nile" },
];

/// Tworzy venv w `venv_dir()` (jeśli jeszcze nie istnieje) i instaluje do
/// niego te z `LAUNCHER_TOOLS`, których jeszcze nie ma w `PATH`. Postęp
/// (kolejne linie stdout/stderr `pip`, plus który krok aktualnie trwa)
/// strumieniowany jest do frontendu przez `hacker-mode://first-run-progress`,
/// a końcowy status przez `hacker-mode://first-run-finished` — ten sam
/// wzorzec co `launcher::install_game`.
#[tauri::command]
pub fn install_launcher_tools(app: AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let emit_line = |line: String| {
            let _ = app.emit("hacker-mode://first-run-progress", serde_json::json!({ "line": line }));
        };

        let venv = venv_dir();
        if !venv.join("bin").join("pip").is_file() && !venv.join("bin").join("pip3").is_file() {
            emit_line(format!("Tworzę środowisko Python w {}…", venv.display()));
            let python = if which::which("python3").is_ok() { "python3" } else { "python" };
            match Command::new(python).args(["-m", "venv", &venv.to_string_lossy()]).output() {
                Ok(out) if out.status.success() => {}
                Ok(out) => {
                    let msg = String::from_utf8_lossy(&out.stderr).into_owned();
                    emit_line(format!("Nie udało się utworzyć venv: {msg}"));
                    let _ = app.emit("hacker-mode://first-run-finished", serde_json::json!({ "ok": false }));
                    return;
                }
                Err(e) => {
                    emit_line(format!("Nie znaleziono `{python}` — zainstaluj Pythona 3, żeby kontynuować ({e})."));
                    let _ = app.emit("hacker-mode://first-run-finished", serde_json::json!({ "ok": false }));
                    return;
                }
            }
        }

        let pip = venv.join("bin").join("pip");
        let mut all_ok = true;

        for tool in LAUNCHER_TOOLS {
            if which::which(tool.id).is_ok() {
                emit_line(format!("{} — już dostępne, pomijam.", tool.label));
                continue;
            }
            emit_line(format!("Instaluję {}…", tool.label));

            let mut cmd = Command::new(&pip);
            cmd.args(["install", "--disable-pip-version-check", tool.pip_target]);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    emit_line(format!("Nie udało się uruchomić pip dla {}: {e}", tool.label));
                    all_ok = false;
                    continue;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                use std::io::{BufRead, BufReader};
                for line in BufReader::new(stdout).lines().flatten() {
                    emit_line(line);
                }
            }
            let status = child.wait();
            let ok = status.map(|s| s.success()).unwrap_or(false);
            if !ok {
                emit_line(format!("Instalacja {} nie powiodła się.", tool.label));
                all_ok = false;
            } else {
                emit_line(format!("{} zainstalowane.", tool.label));
            }
        }

        // Świeżo zainstalowane narzędzia trafiły do `venv/bin` — dopisujemy
        // ten katalog do `PATH` TERAZ (a nie dopiero przy następnym starcie
        // aplikacji), żeby `which::which`/`Command::new("legendary")` w
        // reszcie tej samej sesji już je widziały.
        prepend_venv_to_path();

        let _ = app.emit("hacker-mode://first-run-finished", serde_json::json!({ "ok": all_ok }));
    });

    Ok(())
}
