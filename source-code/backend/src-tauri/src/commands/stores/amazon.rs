use super::{tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
pub struct AmazonProvider;

/// `nile auth --login` blokuje do czasu zakończenia logowania przez
/// użytkownika w przeglądarce. `login_start` (wymagane przez trait
/// `StoreProvider`) musi zwrócić się szybko z samym URL-em, więc proces
/// jest "odkładany" tutaj, a jego zakończenia pilnuje osobny wątek
/// uruchamiany z `store_login::open_login_window` (tam, gdzie mamy
/// `AppHandle` do wyemitowania eventu z wynikiem) — patrz `take_pending_child`.
static PENDING_LOGIN_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

/// Zabiera odłożony proces logowania (jeśli jakiś czeka) — wywoływane
/// dokładnie raz po `login_start`, z wątku, który go doczeka i zgłosi
/// wynik. Zwraca `None`, jeśli nie ma żadnego oczekującego procesu (np.
/// `login_start` nie został jeszcze wywołany albo już go ktoś odebrał).
pub fn take_pending_login_child() -> Option<Child> {
    PENDING_LOGIN_CHILD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .take()
}

#[derive(Debug, Deserialize)]
struct NileInstalledGame {
    id: String,
    #[serde(alias = "title")]
    product_title: Option<String>,
    path: Option<String>,
}

impl StoreProvider for AmazonProvider {
    fn platform(&self) -> Platform {
        Platform::Amazon
    }

    fn is_available(&self) -> bool {
        tool_available("nile")
    }

    fn is_logged_in(&self) -> bool {
        Command::new("nile")
            .args(["library", "list"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        let mut child = Command::new("nile")
            .args(["auth", "--login"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| StoreError::ExecFailed("nile".into(), e.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| StoreError::ExecFailed("nile".into(), "brak stdout".into()))?;
        let reader = BufReader::new(stdout);

        // `nile auth --login` sam prowadzi cały flow logowania i (wg jego
        // dokumentacji) blokuje do czasu zakończenia przez użytkownika w
        // przeglądarce — celowo NIE czekamy na zakończenie procesu tutaj
        // (`child.wait()`), tylko wyciągamy pierwszy URL, jaki wypisze, i
        // pozwalamy procesowi kontynuować w tle. `is_logged_in` odpytywane
        // z UI po zamknięciu okna logowania wykryje sukces.
        let mut found_url = None;
        for line in reader.lines().take(20).flatten() {
            if let Some(start) = line.find("https://") {
                found_url = Some(line[start..].trim().to_string());
                break;
            }
        }

        match found_url {
            Some(url) => {
                // Odkładamy proces do doczekania przez `store_login.rs` —
                // NIE porzucamy go bez śledzenia wyniku (to był błąd w
                // poprzedniej wersji: proces stawał się sierotą, a UI nie
                // dostawało żadnej informacji o niepowodzeniu logowania).
                *PENDING_LOGIN_CHILD.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(child);
                Ok(LoginFlow {
                    url: Some(url),
                    instructions: "Zaloguj się na swoje konto Amazon w otwartym oknie. Po \
                        zakończeniu po prostu zamknij to okno — Hacker Mode sprawdzi, czy \
                        logowanie się powiodło."
                        .to_string(),
                    needs_code: false,
                })
            }
            None => Err(StoreError::ParseError(
                "nile auth --login".into(),
                "nie znaleziono URL-a logowania na wyjściu `nile` — sprawdź ręcznie w \
                 terminalu poleceniem `nile auth --login`"
                    .into(),
            )),
        }
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("nile")
            .args(["library", "list", "--installed", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("nile".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::NotLoggedIn("Amazon Games".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<NileInstalledGame> = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("nile library list".into(), e.to_string()))?;

        let games = parsed
            .into_iter()
            .map(|entry| Game {
                title: entry.product_title.clone().unwrap_or_else(|| format!("Gra Amazon #{}", entry.id)),
                id: entry.id,
                platform: Platform::Amazon,
                installed: true,
                install_dir: entry.path,
                cover_path: None,
                playtime_minutes: None,
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["install", game_id]);
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["uninstall", game_id]);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("nile");
        cmd.args(["launch", game_id]);
        Ok(cmd)
    }
}
