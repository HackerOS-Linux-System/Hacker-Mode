use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hacker_mode_ipc::*;

/// The client reads `HACKER_MODE_IPC_SOCKET` from the process environment,
/// which is global - serialize all tests in this file.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct MockState {
    /// Calls the mock received, in order.
    calls: Vec<IpcCall>,
    /// Windows returned by `ListWindows` (mutated by the test thread).
    windows: Vec<WindowInfo>,
}

fn window(id: u64, app_id: &str) -> WindowInfo {
    serde_json::from_value(serde_json::json!({
        "id": id, "app_id": app_id, "title": "t", "workspace": 1,
        "focused": false, "minimized": false, "maximized": false, "floating": false
    }))
    .expect("WindowInfo shape")
}

fn spawn_mock(path: &std::path::Path, state: Arc<Mutex<MockState>>) {
    let listener = UnixListener::bind(path).expect("bind mock socket");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let state = state.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                let req: IpcRequest = serde_json::from_str(line.trim()).expect("valid request");
                let mut st = state.lock().unwrap();
                st.calls.push(req.call.clone());
                let outcome = match req.call {
                    IpcCall::Ping => IpcOutcome::Ok { result: IpcResult::Pong },
                    IpcCall::ListWindows => IpcOutcome::Ok { result: IpcResult::Windows(st.windows.clone()) },
                    IpcCall::CloseWindow { .. } => IpcOutcome::Err { message: "nope".into() },
                    _ => IpcOutcome::Ok { result: IpcResult::None },
                };
                let resp = IpcResponse { id: req.id, outcome };
                let mut out = serde_json::to_string(&resp).unwrap();
                out.push('\n');
                let mut s = stream;
                let _ = s.write_all(out.as_bytes());
            });
        }
    });
}

struct Fixture {
    _dir: std::path::PathBuf,
    state: Arc<Mutex<MockState>>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn fixture(name: &str) -> Fixture {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("hm-ipc-test-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("mock.sock");
    let _ = std::fs::remove_file(&sock);
    let state = Arc::new(Mutex::new(MockState::default()));
    spawn_mock(&sock, state.clone());
    std::env::set_var("HACKER_MODE_IPC_SOCKET", &sock);
    Fixture { _dir: dir, state, _guard: guard }
}

#[test]
fn ping_and_is_running() {
    let _f = fixture("ping");
    assert!(is_running());
    assert!(matches!(call(IpcCall::Ping, Duration::from_secs(1)), Ok(IpcResult::Pong)));
}

#[test]
fn rejected_request_maps_to_error() {
    let _f = fixture("rejected");
    let err = call(IpcCall::CloseWindow { id: 1 }, Duration::from_secs(1)).unwrap_err();
    assert!(matches!(err, HackerModeIpcError::Rejected(m) if m == "nope"));
}

#[test]
fn enter_and_exit_wrapper_target_the_shell_window() {
    let f = fixture("wrapper");
    f.state.lock().unwrap().windows = vec![window(5, "other.app"), window(9, SHELL_APP_ID)];
    enter_wrapper(Duration::from_secs(1)).unwrap();
    exit_wrapper(Duration::from_secs(1)).unwrap();
    let calls = f.state.lock().unwrap().calls.clone();
    assert!(calls.iter().any(|c| matches!(c, IpcCall::MinimizeWindow { id: 9 })));
    assert!(calls.iter().any(|c| matches!(c, IpcCall::UnminimizeWindow { id: 9 })));
    assert!(calls.iter().any(|c| matches!(c, IpcCall::FocusWindow { id: 9 })));
}

#[test]
fn enter_wrapper_without_shell_window_reports_it() {
    let f = fixture("noshell");
    f.state.lock().unwrap().windows = vec![window(5, "other.app")];
    assert!(matches!(enter_wrapper(Duration::from_secs(1)), Err(HackerModeIpcError::ShellWindowNotFound)));
}

#[test]
fn maximize_next_new_window_skips_known_and_shell_windows() {
    let f = fixture("maximize");
    f.state.lock().unwrap().windows = vec![window(1, "old.app"), window(2, SHELL_APP_ID), window(3, "new.game")];
    let id = maximize_next_new_window(&[1], Duration::from_secs(1), Duration::from_millis(20)).unwrap();
    assert_eq!(id, 3);
    let calls = f.state.lock().unwrap().calls.clone();
    assert!(calls.iter().any(|c| matches!(c, IpcCall::MaximizeWindow { id: 3, maximized: true })));
}

#[test]
fn maximize_next_new_window_times_out_when_nothing_new_appears() {
    let f = fixture("timeout");
    f.state.lock().unwrap().windows = vec![window(1, "old.app")];
    let err = maximize_next_new_window(&[1], Duration::from_millis(150), Duration::from_millis(30)).unwrap_err();
    assert!(matches!(err, HackerModeIpcError::NewWindowTimedOut(_)));
}

#[test]
fn missing_socket_reports_not_running() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("HACKER_MODE_IPC_SOCKET", "/nonexistent/hm-test.sock");
    assert!(matches!(call(IpcCall::Ping, Duration::from_millis(100)), Err(HackerModeIpcError::NotRunning(_))));
    assert!(!is_running());
}

/// Real-binary smoke test. Ignored by default; run with
/// `COMPHWDE_BIN=/path/to/comphwde cargo test -p hacker-mode-ipc -- --ignored`
/// inside a session where `comphwde --extern-hacker-mode` can start
/// (nested under an existing Wayland/X11 session is fine).
#[test]
#[ignore]
fn real_comphwde_ping() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("HACKER_MODE_IPC_SOCKET");
    let bin = std::env::var("COMPHWDE_BIN").expect("set COMPHWDE_BIN to the comphwde binary");
    let mut child = std::process::Command::new(bin).arg("--extern-hacker-mode").spawn().expect("spawn comphwde");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut ok = false;
    while std::time::Instant::now() < deadline {
        if is_running() {
            ok = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = shutdown_compositor(Duration::from_secs(2));
    let _ = child.kill();
    let _ = child.wait();
    assert!(ok, "comphwde never answered Ping on {:?}", socket_path());
}
