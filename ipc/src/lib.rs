use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// Wire protocol (vendored copy of comphwde's --extern-<name> protocol -
// see module docs above for why this is a copy, not a shared crate).
// Field/variant names and serde attributes below MUST match comphwde's
// `sde-ipc` crate exactly; this is what keeps the two sides compatible.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: u64,
    #[serde(flatten)]
    pub call: IpcCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum IpcCall {
    Ping,
    LaunchApp { command: String, args: Vec<String> },
    SetWallpaper { path: String },
    ListWindows,
    FocusWindow { id: u64 },
    CloseWindow { id: u64 },
    MinimizeWindow { id: u64 },
    UnminimizeWindow { id: u64 },
    MaximizeWindow { id: u64, maximized: bool },
    ToggleFloatingWindow { id: u64 },
    ListWorkspaces,
    SwitchWorkspace { id: u32 },
    MoveWindowToWorkspace { id: u64, workspace: u32 },
    SetTiling { workspace: u32, enabled: bool },
    PinSurface { app_id: String, edge: PinnedEdge, thickness_px: u32 },
    ReloadConfig,
    ListOutputs,
    Shutdown,
    Subscribe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinnedEdge {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: u64,
    #[serde(flatten)]
    pub outcome: IpcOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcOutcome {
    Ok { result: IpcResult },
    Err { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IpcResult {
    None,
    Pong,
    Windows(Vec<WindowInfo>),
    Workspaces(Vec<WorkspaceInfo>),
    Outputs(Vec<OutputInfo>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub workspace: u32,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub floating: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: u32,
    pub name: String,
    pub window_count: u32,
    pub tiling_enabled: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputInfo {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub refresh_mhz: i32,
    pub scale: f64,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum IpcEvent {
    Windows(Vec<WindowInfo>),
    Workspaces(Vec<WorkspaceInfo>),
    CompositorShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEventMessage {
    pub event: IpcEvent,
}

#[derive(Debug, thiserror::Error)]
pub enum HackerModeIpcError {
    #[error("comphwde nie działa w trybie --extern-hacker-mode (brak gniazda {0:?})")]
    NotRunning(PathBuf),
    #[error("błąd i/o w komunikacji z comphwde: {0}")]
    Io(#[from] std::io::Error),
    #[error("niepoprawna odpowiedź od comphwde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("comphwde odrzuciło żądanie: {0}")]
    Rejected(String),
    #[error("id odpowiedzi {got} nie zgadza się z id żądania {expected}")]
    IdMismatch { expected: u64, got: u64 },
    #[error("nie znaleziono własnego okna powłoki Hacker Mode (app_id={SHELL_APP_ID}) w ListWindows")]
    ShellWindowNotFound,
    #[error("nie wykryto nowego okna w ciągu {0:?} od uruchomienia procesu")]
    NewWindowTimedOut(Duration),
}

// ---------------------------------------------------------------------
// Hacker-Mode-specific naming/constants
// ---------------------------------------------------------------------

/// The `--extern-<name>` name Hacker Mode's session always uses.
pub const EXTERN_NAME: &str = "hacker-mode";

/// The `app_id`/`identifier` Hacker Mode's own shell window reports over
/// `ListWindows` - `tauri.conf.json`'s `identifier`
/// (`com.hackeros.hackermode`).
pub const SHELL_APP_ID: &str = "com.hackeros.hackermode";

/// Default timeout for a single request/response round-trip.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// `$XDG_RUNTIME_DIR/sde`, falling back to `/tmp/sde-<uid>` if
/// `XDG_RUNTIME_DIR` isn't set. Named `.../sde` (not `.../hacker-mode`)
/// because that's the shared runtime directory comphwde itself uses for
/// *every* `--extern-<name>` socket, regardless of which target - see
/// [`socket_path`] and comphwde's own `sde_ipc::runtime_dir()`, which
/// this must match exactly.
fn runtime_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("sde");
        }
    }
    let uid = unsafe { libc_getuid() };
    PathBuf::from(format!("/tmp/sde-{uid}"))
}

unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

/// Path to the control socket for `comphwde --extern-hacker-mode`.
pub fn socket_path() -> PathBuf {
    runtime_dir().join("comphwde-hacker-mode.sock")
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Sends one request to `comphwde --extern-hacker-mode` and waits (up to
/// `timeout`) for the matching response. Opens a fresh connection per
/// call, same as comphwde's own `sde-ipc` client does for SDE.
pub fn call(request: IpcCall, timeout: Duration) -> Result<IpcResult, HackerModeIpcError> {
    let path = socket_path();
    if !path.exists() {
        return Err(HackerModeIpcError::NotRunning(path));
    }

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let req = IpcRequest { id, call: request };

    let mut stream = UnixStream::connect(&path)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    stream.write_all(line.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let response: IpcResponse = serde_json::from_str(response_line.trim())?;
    if response.id != id {
        return Err(HackerModeIpcError::IdMismatch { expected: id, got: response.id });
    }
    match response.outcome {
        IpcOutcome::Ok { result } => Ok(result),
        IpcOutcome::Err { message } => Err(HackerModeIpcError::Rejected(message)),
    }
}

/// True if a `comphwde --extern-hacker-mode` compositor is currently
/// reachable.
pub fn is_running() -> bool {
    matches!(call(IpcCall::Ping, Duration::from_millis(300)), Ok(IpcResult::Pong))
}

/// A live event stream from `comphwde --extern-hacker-mode`.
pub struct Subscription {
    reader: BufReader<UnixStream>,
}

impl Subscription {
    /// Blocks until the next event arrives, forever (no read timeout) -
    /// intended for a dedicated background thread.
    pub fn recv(&mut self) -> Result<IpcEvent, HackerModeIpcError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(HackerModeIpcError::Io(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "hacker-mode-ipc subscription closed")));
        }
        let msg: IpcEventMessage = serde_json::from_str(line.trim())?;
        Ok(msg.event)
    }
}

/// Opens a [`Subscription`] to `comphwde --extern-hacker-mode`.
pub fn subscribe() -> Result<Subscription, HackerModeIpcError> {
    let path = socket_path();
    if !path.exists() {
        return Err(HackerModeIpcError::NotRunning(path));
    }

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let req = IpcRequest { id, call: IpcCall::Subscribe };

    let stream = UnixStream::connect(&path)?;
    stream.set_write_timeout(Some(Duration::from_millis(800)))?;

    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    (&stream).write_all(line.as_bytes())?;
    (&stream).flush()?;

    let mut reader = BufReader::new(stream);
    let mut ack_line = String::new();
    reader.read_line(&mut ack_line)?;
    let ack: IpcResponse = serde_json::from_str(ack_line.trim())?;
    if ack.id != id {
        return Err(HackerModeIpcError::IdMismatch { expected: id, got: ack.id });
    }
    match ack.outcome {
        IpcOutcome::Ok { .. } => {
            reader.get_ref().set_read_timeout(None)?;
            Ok(Subscription { reader })
        }
        IpcOutcome::Err { message } => Err(HackerModeIpcError::Rejected(message)),
    }
}

// ---------------------------------------------------------------------
// Hacker-Mode-specific conveniences built on the primitives above
// ---------------------------------------------------------------------

/// Asks comphwde to end the `--extern-hacker-mode` session cleanly - used
/// by the shell's own shutdown/window-close handler so
/// `scripts/hacker-mode-session` (which `wait`s on the compositor
/// process) exits as soon as the shell does.
pub fn shutdown_compositor(timeout: Duration) -> Result<(), HackerModeIpcError> {
    call(IpcCall::Shutdown, timeout).map(|_| ())
}

/// Asks comphwde to spawn `command` (with `args`) as a proper Wayland/
/// XWayland client of this `--extern-hacker-mode` session - i.e. with a
/// correctly-set `WAYLAND_DISPLAY` for *this* compositor instance. This
/// is how `scripts/hacker-mode-session` starts the shell itself, instead
/// of trying to guess/export `WAYLAND_DISPLAY` by hand (which a session
/// script fundamentally cannot do reliably - see that script's comments).
pub fn launch_shell(command: &str, args: &[String]) -> Result<(), HackerModeIpcError> {
    call(IpcCall::LaunchApp { command: command.to_string(), args: args.to_vec() }, DEFAULT_TIMEOUT).map(|_| ())
}

/// Finds Hacker Mode's own shell window (matching [`SHELL_APP_ID`]) in the
/// current `ListWindows` result.
pub fn find_shell_window(timeout: Duration) -> Result<Option<WindowInfo>, HackerModeIpcError> {
    match call(IpcCall::ListWindows, timeout)? {
        IpcResult::Windows(windows) => Ok(windows.into_iter().find(|w| w.app_id == SHELL_APP_ID)),
        _ => Ok(None),
    }
}

/// Enter "wrapper mode": get Hacker Mode's own shell window out of the way
/// right before spawning a game/launcher, so the newly-launched window has
/// the screen to itself. Built from plain `ListWindows` + `MinimizeWindow`
/// - see module docs for why comphwde doesn't need a dedicated call for
/// this.
pub fn enter_wrapper(timeout: Duration) -> Result<(), HackerModeIpcError> {
    let shell = find_shell_window(timeout)?.ok_or(HackerModeIpcError::ShellWindowNotFound)?;
    call(IpcCall::MinimizeWindow { id: shell.id }, timeout)?;
    Ok(())
}

/// Leave "wrapper mode": restore and focus Hacker Mode's own shell window.
pub fn exit_wrapper(timeout: Duration) -> Result<(), HackerModeIpcError> {
    let shell = find_shell_window(timeout)?.ok_or(HackerModeIpcError::ShellWindowNotFound)?;
    call(IpcCall::UnminimizeWindow { id: shell.id }, timeout)?;
    call(IpcCall::FocusWindow { id: shell.id }, timeout)?;
    Ok(())
}

/// Polls `ListWindows` (every `poll_interval`, up to `timeout` total) for
/// a window whose id isn't in `known_ids`, and maximizes the first one it
/// finds - used right after spawning a game/launcher process so its
/// window ends up fullscreen without Hacker Mode having to know the
/// game's `app_id`/window title in advance. Returns the id of the window
/// it maximized, if any.
pub fn maximize_next_new_window(
    known_ids: &[u64],
    timeout: Duration,
    poll_interval: Duration,
) -> Result<u64, HackerModeIpcError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let IpcResult::Windows(windows) = call(IpcCall::ListWindows, DEFAULT_TIMEOUT)? {
            if let Some(w) = windows.into_iter().find(|w| !known_ids.contains(&w.id) && w.app_id != SHELL_APP_ID) {
                call(IpcCall::MaximizeWindow { id: w.id, maximized: true }, DEFAULT_TIMEOUT)?;
                return Ok(w.id);
            }
        }
        if Instant::now() >= deadline {
            return Err(HackerModeIpcError::NewWindowTimedOut(timeout));
        }
        std::thread::sleep(poll_interval);
    }
}

/// Snapshot of currently-known window ids - pass the result to
/// [`maximize_next_new_window`] as `known_ids` right before spawning a
/// process, so it can tell "new" windows apart from ones that already
/// existed.
pub fn known_window_ids(timeout: Duration) -> Result<Vec<u64>, HackerModeIpcError> {
    match call(IpcCall::ListWindows, timeout)? {
        IpcResult::Windows(windows) => Ok(windows.into_iter().map(|w| w.id).collect()),
        _ => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrips_through_json() {
        let req = IpcRequest { id: 42, call: IpcCall::FocusWindow { id: 7 } };
        let json = serde_json::to_string(&req).unwrap();
        let back: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        matches!(back.call, IpcCall::FocusWindow { id: 7 });
    }

    #[test]
    fn response_roundtrips_through_json() {
        let resp = IpcResponse { id: 1, outcome: IpcOutcome::Ok { result: IpcResult::Pong } };
        let json = serde_json::to_string(&resp).unwrap();
        let back: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 1);
        matches!(back.outcome, IpcOutcome::Ok { result: IpcResult::Pong });
    }

    #[test]
    fn wire_shape_matches_comphwde_expectations() {
        // Sanity check on the exact JSON shape, since this crate
        // deliberately duplicates (rather than shares) comphwde's
        // protocol definition - see module docs.
        let req = IpcRequest { id: 1, call: IpcCall::FocusWindow { id: 7 } };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["method"], "focus_window");
        assert_eq!(json["params"]["id"], 7);
    }
}
