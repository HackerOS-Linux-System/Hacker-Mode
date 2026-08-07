pub mod amazon;
pub mod epic;
pub mod gog;
pub mod lutris;
pub mod steam;

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Epic,
    Gog,
    Amazon,
    Lutris,
}

impl Platform {
    pub fn label(&self) -> &'static str {
        match self {
            Platform::Steam => "Steam",
            Platform::Epic => "Epic Games",
            Platform::Gog => "GOG",
            Platform::Amazon => "Amazon Games",
            Platform::Lutris => "Lutris",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// Unikalny w obrębie danej platformy identyfikator (np. Steam AppID).
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub installed: bool,
    pub install_dir: Option<String>,
    /// Ścieżka do okładki/artworku, jeśli dostępna lokalnie w cache sklepu.
    pub cover_path: Option<String>,
    /// Czas gry w minutach, jeśli platforma to udostępnia.
    pub playtime_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetails {
    pub description: String,
    pub screenshots: Vec<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum StoreError {
    #[error("narzędzie `{0}` nie jest zainstalowane w systemie")]
    ToolNotInstalled(String),
    #[error("błąd uruchomienia narzędzia `{0}`: {1}")]
    ExecFailed(String, String),
    #[error("nie udało się sparsować danych z `{0}`: {1}")]
    ParseError(String, String),
    #[error("nie zalogowano do platformy {0}")]
    NotLoggedIn(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFlow {
    /// URL do otwarcia w oknie webview osadzonym w Hacker Mode (ta sama
    /// sesja — bez zewnętrznej przeglądarki), na którym użytkownik loguje
    /// się do swojego konta.
    pub url: Option<String>,
    /// Instrukcja pokazywana użytkownikowi w UI (np. co zrobić po
    /// zalogowaniu, skąd skopiować kod).
    pub instructions: String,
    /// Czy po zalogowaniu trzeba wywołać `login_submit` z kodem/tokenem
    /// wklejonym przez użytkownika (flow "authorization code"), czy
    /// logowanie kończy się samo (np. Steam — obsługiwane przez sam
    /// klient Steam, bez udziału Hacker Mode).
    pub needs_code: bool,
}

/// Wspólny interfejs sklepu/launchera gier.
pub trait StoreProvider {
    fn platform(&self) -> Platform;

    /// Czy narzędzie/klient wymagany do obsługi tej platformy jest dostępny
    /// w systemie (np. `legendary`, `gogdl`, `nile`, `lutris`, `flatpak`
    /// ze Steamem). Jeśli `false`, sklep jest po prostu pomijany przy
    /// agregacji zamiast wywalać cały wynik.
    fn is_available(&self) -> bool;

    /// Czy użytkownik jest zalogowany do tej platformy (na tyle, żeby
    /// `list_games` miało sens). Domyślnie `true` — nadpisywane przez
    /// dostawców, którzy tego wymagają (Epic/GOG/Amazon).
    fn is_logged_in(&self) -> bool {
        true
    }

    /// Rozpoczyna proces logowania — zwraca URL do pokazania w osadzonym
    /// webview oraz instrukcję dla użytkownika.
    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "logowanie nie jest wymagane/obsługiwane dla tej platformy".into(),
        ))
    }

    /// Kończy proces logowania kodem autoryzacyjnym wklejonym przez
    /// użytkownika po zalogowaniu się na stronie z `login_start`.
    fn login_submit(&self, _code: &str) -> Result<(), StoreError> {
        Err(StoreError::ToolNotInstalled(
            "logowanie kodem nie jest wymagane/obsługiwane dla tej platformy".into(),
        ))
    }

    /// Zwraca listę zainstalowanych (i ewentualnie dostępnych do
    /// zainstalowania) gier. Implementacje NIE powinny panikować przy
    /// braku danych — zwracają pustą listę / błąd, który agregator
    /// zamienia na ostrzeżenie w logu.
    fn list_games(&self) -> Result<Vec<Game>, StoreError>;

    /// Buduje komendę instalującą grę o danym identyfikatorze. Domyślnie
    /// nieobsługiwane (np. Lutris, gdzie instalacja idzie przez własny
    /// system skryptów instalacyjnych, a nie prosty `install <id>`).
    fn install_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "instalacja z poziomu Hacker Mode nie jest obsługiwana dla tej platformy".into(),
        ))
    }

    /// Buduje komendę odinstalowującą grę. Domyślnie nieobsługiwane.
    fn uninstall_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "odinstalowanie z poziomu Hacker Mode nie jest obsługiwane dla tej platformy".into(),
        ))
    }

    /// Zbuduj komendę uruchamiającą daną grę. Zwracany `Command` jest
    /// dalej przekazywany do `commands::launcher`, które odpowiada za
    /// tryb wrapper / IPC z kompozytorem / logowanie.
    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError>;
}

/// Agreguje biblioteki ze wszystkich dostępnych platform. Błąd pojedynczego
/// dostawcy nie przerywa całości — jest logowany i pomijany, tak by np.
/// brak konta Amazon Games nie blokował widoku biblioteki Steam.
///
/// Uwaga wydajnościowa: pobieranie okładek dla Epic/GOG (patrz
/// `epic.rs`/`gog.rs`) jest synchroniczne i sieciowe — przy dużej bibliotece
/// bez lokalnego cache (pierwsze uruchomienie) `list_games` może zauważalnie
/// zwolnić. Do rozważenia w przyszłości: równoległe pobieranie (np. przez
/// `rayon` albo pulę wątków) zamiast pobierania okładka-po-okładce.
pub fn list_all_games(steam_credentials: Option<(&str, &str)>) -> Vec<Game> {
    let providers: Vec<Box<dyn StoreProvider>> = vec![
        Box::new(steam::SteamProvider::default()),
        Box::new(epic::EpicProvider::default()),
        Box::new(gog::GogProvider::default()),
        Box::new(amazon::AmazonProvider::default()),
        Box::new(lutris::LutrisProvider::default()),
    ];

    let mut all_games = Vec::new();
    for provider in providers {
        if !provider.is_available() {
            tracing::debug!(platform = ?provider.platform(), "Sklep niedostępny (brak narzędzia), pomijam");
            continue;
        }
        match provider.list_games() {
            Ok(mut games) => {
                tracing::info!(platform = ?provider.platform(), count = games.len(), "Wczytano bibliotekę");
                all_games.append(&mut games);
            }
            Err(err) => {
                tracing::warn!(platform = ?provider.platform(), error = %err, "Nie udało się wczytać biblioteki");
            }
        }
    }

    if let Some((api_key, steam_id64)) = steam_credentials {
        let playtimes = steam::fetch_owned_games_playtime(api_key, steam_id64);
        if !playtimes.is_empty() {
            for game in all_games.iter_mut().filter(|g| g.platform == Platform::Steam) {
                if let Some(minutes) = playtimes.get(&game.id) {
                    game.playtime_minutes = Some(*minutes);
                }
            }
        }
    }

    all_games
}

/// Buduje komendę uruchomienia gry, przeszukując odpowiedniego dostawcę wg
/// platformy przekazanej z frontendu (frontend zna platformę, bo to on
/// wyrenderował kartę gry z danych zwróconych przez `list_all_games`).
pub fn build_launch_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().launch_command(game_id),
        Platform::Epic => epic::EpicProvider::default().launch_command(game_id),
        Platform::Gog => gog::GogProvider::default().launch_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().launch_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().launch_command(game_id),
    }
}

pub fn build_install_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().install_command(game_id),
        Platform::Epic => epic::EpicProvider::default().install_command(game_id),
        Platform::Gog => gog::GogProvider::default().install_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().install_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().install_command(game_id),
    }
}

pub fn build_uninstall_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().uninstall_command(game_id),
        Platform::Epic => epic::EpicProvider::default().uninstall_command(game_id),
        Platform::Gog => gog::GogProvider::default().uninstall_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().uninstall_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().uninstall_command(game_id),
    }
}

pub fn is_logged_in(platform: Platform) -> bool {
    match platform {
        Platform::Steam => steam::SteamProvider::default().is_logged_in(),
        Platform::Epic => epic::EpicProvider::default().is_logged_in(),
        Platform::Gog => gog::GogProvider::default().is_logged_in(),
        Platform::Amazon => amazon::AmazonProvider::default().is_logged_in(),
        Platform::Lutris => lutris::LutrisProvider::default().is_logged_in(),
    }
}

pub fn login_start(platform: Platform) -> Result<LoginFlow, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().login_start(),
        Platform::Epic => epic::EpicProvider::default().login_start(),
        Platform::Gog => gog::GogProvider::default().login_start(),
        Platform::Amazon => amazon::AmazonProvider::default().login_start(),
        Platform::Lutris => lutris::LutrisProvider::default().login_start(),
    }
}

pub fn login_submit(platform: Platform, code: &str) -> Result<(), StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().login_submit(code),
        Platform::Epic => epic::EpicProvider::default().login_submit(code),
        Platform::Gog => gog::GogProvider::default().login_submit(code),
        Platform::Amazon => amazon::AmazonProvider::default().login_submit(code),
        Platform::Lutris => lutris::LutrisProvider::default().login_submit(code),
    }
}

/// Dociąga rozszerzone informacje o grze (opis, zrzuty ekranu) do widoku
/// szczegółów w UI. Obecnie zaimplementowane tylko dla Steam (publiczne,
/// nieuwierzytelnione Store API) — Epic/GOG/Amazon wymagałyby osobnej
/// integracji z ich katalogami, co jest sporym, oddzielnym zadaniem (patrz
/// README, sekcja "Status komponentów").
pub fn fetch_game_details(platform: Platform, game_id: &str) -> Option<GameDetails> {
    match platform {
        Platform::Steam => steam::fetch_store_details(game_id),
        _ => None,
    }
}

/// Pomocnicza funkcja: sprawdza obecność narzędzia w `PATH`.
pub(crate) fn tool_available(name: &str) -> bool {
    which::which(name).is_ok()
}
