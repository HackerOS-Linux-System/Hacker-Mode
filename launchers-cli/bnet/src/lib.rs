use std::path::{Path, PathBuf};
use std::process::Command;

pub mod ribbit;
pub use ribbit::{query_product_versions, ProductVersion, RibbitError, KNOWN_PRODUCT_CODES};

/// Domyślna lokalizacja prefiksu Wine z Battle.net względem `$HOME` —
/// zgodna z `$GAMEDIR` sugerowanym przez oficjalny skrypt instalacyjny
/// Lutrisa dla `game_slug: battlenet`
/// (<https://lutris.net/games/install/37360/view>). To tylko DOMYŚLNA
/// propozycja Lutrisa, nie jedyne możliwe miejsce — stąd [`find_prefix`]
/// przyjmuje też jawnie podaną ścieżkę (np. z ustawień Hacker Mode) jako
/// pierwszeństwo.
pub const KNOWN_PREFIX_CANDIDATES: &[&str] = &["Games/battlenet", "Games/battle-net", "Games/Battle.net"];

/// Ścieżka do klienta Battle.net względem prefiksu Wine — dokładnie zgodna
/// z `game.exe` w oficjalnym skrypcie instalacyjnym Lutrisa
/// (<https://lutris.net/games/install/37360/view>).
pub const LAUNCHER_RELATIVE_PATH: &str = "drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe";

/// Katalog, w którym Wine trzyma 32-bitowe "Program Files" tego prefiksu —
/// Battle.net instaluje każdą grę jako osobny podkatalog obok siebie tutaj
/// (np. `World of Warcraft`, `Diablo III`, `Overwatch`) zamiast we
/// wspólnym katalogu gier (inaczej niż EA app) — stąd [`list_installed_games`]
/// filtruje ten katalog listą wykluczeń [`NON_GAME_DIRS`] zamiast brać
/// wszystko bez wyjątku.
pub const PROGRAM_FILES_X86_RELATIVE_DIR: &str = "drive_c/Program Files (x86)";

/// Podkatalogi w [`PROGRAM_FILES_X86_RELATIVE_DIR`], które na pewno NIE są
/// grą — sam klient Battle.net i standardowe foldery systemowe Windows
/// obecne w każdym świeżym prefiksie Wine.
pub const NON_GAME_DIRS: &[&str] = &[
    "Battle.net",
    "Common Files",
    "InstallShield Installation Information",
    "Internet Explorer",
    "Windows NT",
    "Windows Mail",
    "Windows Media Player",
    "Windows Photo Viewer",
    "WindowsPowerShell",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledGame {
    /// Nazwa katalogu instalacji gry — jedyny stabilny identyfikator, jaki
    /// mamy bez dostępu do prywatnego API Battle.net.
    pub id: String,
    pub title: String,
    pub install_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BnetError {
    #[error("nie znaleziono prefiksu Wine z zainstalowanym Battle.net (sprawdzono: {0:?})")]
    PrefixNotFound(Vec<PathBuf>),
    #[error("polecenie `wine` nie jest zainstalowane w systemie")]
    WineNotInstalled,
    #[error("nie udało się odczytać katalogu `{0}`: {1}")]
    ReadDirFailed(PathBuf, String),
    /// Patrz `ea_cli::EaError::InvalidGameId` — ten sam powód, ten sam
    /// zakres walidacji, tylko dla katalogu gry Battle.net.
    #[error("nieprawidłowy identyfikator gry: `{0}`")]
    InvalidGameId(String),
    /// Patrz `ea_cli::EaError::GameDirNotFound`.
    #[error("nie znaleziono katalogu gry `{0}` w `{1}`")]
    GameDirNotFound(String, PathBuf),
    /// Zwracane przez [`resolve_game_dir`], gdy `game_id` pasuje do
    /// [`NON_GAME_DIRS`] — czyli odnosi się do samego klienta Battle.net
    /// albo folderu systemowego Wine, nie do żadnej gry. Bez tej
    /// dodatkowej blokady (ponad walidację path traversal, wspólną z
    /// `ea_cli`) dałoby się "odinstalować" np. `Battle.net` (sam klient)
    /// przez to samo API co zwykłą grę.
    #[error("`{0}` to nie gra (folder systemowy/klienta Battle.net)")]
    NotAGame(String),
}

/// Czy `wine` jest w ogóle dostępny w systemie — warunek konieczny (ale
/// niewystarczający, patrz [`find_prefix`]) do jakiejkolwiek integracji z Battle.net.
pub fn is_available() -> bool {
    which::which("wine").is_ok()
}

/// Szuka prefiksu Wine z zainstalowanym klientem Battle.net. Kolejność
/// sprawdzania: jawnie podana ścieżka (np. z ustawień Hacker Mode), potem
/// [`KNOWN_PREFIX_CANDIDATES`] względem katalogu domowego. Prefiks uznajemy
/// za trafiony, jeśli [`LAUNCHER_RELATIVE_PATH`] istnieje w nim jako plik.
pub fn find_prefix(explicit: Option<&Path>) -> Result<PathBuf, BnetError> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = explicit {
        candidates.push(p.to_path_buf());
    }
    if let Some(home) = dirs::home_dir() {
        for rel in KNOWN_PREFIX_CANDIDATES {
            candidates.push(home.join(rel));
        }
    }

    for candidate in &candidates {
        if candidate.join(LAUNCHER_RELATIVE_PATH).is_file() {
            return Ok(candidate.clone());
        }
    }

    Err(BnetError::PrefixNotFound(candidates))
}

/// Heurystyczna lista zainstalowanych gier Battle.net — po jednym wpisie
/// na podkatalog w [`PROGRAM_FILES_X86_RELATIVE_DIR`], z wykluczeniem
/// [`NON_GAME_DIRS`]. Bez dostępu do API Battle.net nie mamy prawdziwych
/// tytułów/okładek/ID produktów — nazwa katalogu instalacji to zarazem
/// `id` i `title`.
pub fn list_installed_games(prefix: &Path) -> Result<Vec<InstalledGame>, BnetError> {
    let program_files = prefix.join(PROGRAM_FILES_X86_RELATIVE_DIR);
    if !program_files.is_dir() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&program_files)
        .map_err(|e| BnetError::ReadDirFailed(program_files.clone(), e.to_string()))?;

    let mut games = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if NON_GAME_DIRS.iter().any(|excluded| excluded.eq_ignore_ascii_case(&name)) {
            continue;
        }
        games.push(InstalledGame {
            id: name.clone(),
            title: name,
            install_dir: entry.path(),
        });
    }
    games.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(games)
}

/// Rozwiązuje `game_id` (nazwa katalogu, patrz [`InstalledGame::id`]) do
/// pełnej ścieżki katalogu instalacji tej gry wewnątrz
/// [`PROGRAM_FILES_X86_RELATIVE_DIR`] danego prefiksu — patrz
/// `ea_cli::resolve_game_dir` po pełny opis zasady działania i
/// uzasadnienie. Tu dodatkowo odrzuca [`NON_GAME_DIRS`] (patrz
/// [`BnetError::NotAGame`]), żeby nie dało się "odinstalować" samego
/// klienta Battle.net tą samą ścieżką co zwykłą grę.
pub fn resolve_game_dir(prefix: &Path, game_id: &str) -> Result<PathBuf, BnetError> {
    if game_id.is_empty() || game_id.contains('/') || game_id.contains('\\') || game_id.contains("..") {
        return Err(BnetError::InvalidGameId(game_id.to_string()));
    }
    if NON_GAME_DIRS.iter().any(|excluded| excluded.eq_ignore_ascii_case(game_id)) {
        return Err(BnetError::NotAGame(game_id.to_string()));
    }
    let dir = prefix.join(PROGRAM_FILES_X86_RELATIVE_DIR).join(game_id);
    if !dir.is_dir() {
        return Err(BnetError::GameDirNotFound(game_id.to_string(), dir));
    }
    Ok(dir)
}

/// Buduje komendę uruchamiającą sam klient Battle.net wewnątrz jego
/// prefiksu Wine (patrz dokumentacja modułu — uruchomienie konkretnej gry
/// z pominięciem klienta Battle.net nie jest tu obsługiwane, bo
/// wymagałoby nieznanego prywatnego API Blizzarda).
pub fn build_launch_battlenet_command(prefix: &Path) -> Result<Command, BnetError> {
    if !is_available() {
        return Err(BnetError::WineNotInstalled);
    }
    let launcher = prefix.join(LAUNCHER_RELATIVE_PATH);
    if !launcher.is_file() {
        return Err(BnetError::PrefixNotFound(vec![prefix.to_path_buf()]));
    }

    let mut cmd = Command::new("wine");
    cmd.arg(&launcher);
    cmd.env("WINEPREFIX", prefix);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_prefix() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let launcher_dir = dir.path().join("drive_c/Program Files (x86)/Battle.net");
        fs::create_dir_all(&launcher_dir).unwrap();
        fs::write(launcher_dir.join("Battle.net Launcher.exe"), b"").unwrap();
        dir
    }

    #[test]
    fn finds_explicit_prefix_when_launcher_present() {
        let dir = fake_prefix();
        let found = find_prefix(Some(dir.path())).unwrap();
        assert_eq!(found, dir.path());
    }

    #[test]
    fn errors_when_no_launcher_anywhere() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_prefix(Some(dir.path())).is_err());
    }

    #[test]
    fn lists_game_dirs_excluding_known_non_game_dirs() {
        let dir = fake_prefix();
        let pf = dir.path().join(PROGRAM_FILES_X86_RELATIVE_DIR);
        fs::create_dir_all(pf.join("World of Warcraft")).unwrap();
        fs::create_dir_all(pf.join("Diablo III")).unwrap();
        fs::create_dir_all(pf.join("Common Files")).unwrap();

        let games = list_installed_games(dir.path()).unwrap();
        let titles: Vec<_> = games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, vec!["Diablo III", "World of Warcraft"]);
    }

    #[test]
    fn resolve_game_dir_finds_existing_game() {
        let dir = fake_prefix();
        let pf = dir.path().join(PROGRAM_FILES_X86_RELATIVE_DIR);
        fs::create_dir_all(pf.join("Diablo III")).unwrap();

        let resolved = resolve_game_dir(dir.path(), "Diablo III").unwrap();
        assert_eq!(resolved, pf.join("Diablo III"));
    }

    #[test]
    fn resolve_game_dir_rejects_non_game_dirs() {
        let dir = fake_prefix();
        assert!(matches!(
            resolve_game_dir(dir.path(), "Battle.net"),
            Err(BnetError::NotAGame(_))
        ));
        assert!(matches!(
            resolve_game_dir(dir.path(), "Common Files"),
            Err(BnetError::NotAGame(_))
        ));
    }

    #[test]
    fn resolve_game_dir_rejects_path_traversal() {
        let dir = fake_prefix();
        assert!(matches!(
            resolve_game_dir(dir.path(), "../../etc"),
            Err(BnetError::InvalidGameId(_))
        ));
    }
}
