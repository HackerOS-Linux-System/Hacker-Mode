use std::path::{Path, PathBuf};
use std::process::Command;

/// Domyślne lokalizacje prefiksu Wine z EA app, względem `$HOME` — zgodne
/// z `$GAMEDIR` sugerowanym przez najpopularniejsze skrypty instalacyjne
/// Lutrisa dla `game_slug: ea-app`
/// (<https://lutris.net/games/install/36644/view>,
/// <https://lutris.net/games/install/29855/view>). To tylko DOMYŚLNE
/// propozycje Lutrisa, nie jedyne możliwe miejsce — użytkownik mógł przy
/// instalacji wybrać inny katalog, stąd [`find_prefix`] przyjmuje też
/// jawnie podaną ścieżkę (np. z ustawień Hacker Mode) jako pierwszeństwo.
pub const KNOWN_PREFIX_CANDIDATES: &[&str] = &["Games/ea-app", "Games/ea", "Games/EA App"];

/// Ścieżki do launchera EA Desktop względem prefiksu Wine. Różne warianty
/// skryptu instalacyjnego Lutrisa ("Standard" vs "Open Beta") kończą z
/// inną nazwą pliku wykonywalnego, stąd dwie ścieżki do sprawdzenia.
pub const LAUNCHER_RELATIVE_PATHS: &[&str] = &[
    "drive_c/Program Files/Electronic Arts/EA Desktop/EA Desktop/EALauncher.exe",
    "drive_c/Program Files/Electronic Arts/EA Desktop/EA Desktop/EADesktop.exe",
];

/// Katalog, w którym EA Desktop instaluje same gry (wewnątrz tego samego
/// prefiksu Wine, obok samego EA app) — obserwowana konwencja instalacji
/// Windowsowych klientów EA. Każdy podkatalog tu traktowany jest
/// heurystycznie jako jedna zainstalowana gra, patrz [`list_installed_games`].
pub const GAMES_RELATIVE_DIR: &str = "drive_c/Program Files/EA Games";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledGame {
    /// Nazwa katalogu instalacji gry — jedyny stabilny identyfikator,
    /// jaki mamy bez dostępu do API EA (analogicznie do slugu, którego
    /// `LutrisProvider` w Hacker Mode używa, gdy brakuje "prawdziwej" nazwy).
    pub id: String,
    pub title: String,
    pub install_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum EaError {
    #[error("nie znaleziono prefiksu Wine z zainstalowanym EA app (sprawdzono: {0:?})")]
    PrefixNotFound(Vec<PathBuf>),
    #[error("polecenie `wine` nie jest zainstalowane w systemie")]
    WineNotInstalled,
    #[error("nie udało się odczytać katalogu gier `{0}`: {1}")]
    ReadDirFailed(PathBuf, String),
    /// Zwracane przez [`resolve_game_dir`], gdy `game_id` (nazwa katalogu
    /// gry, patrz [`InstalledGame::id`]) wygląda podejrzanie — zawiera
    /// separator ścieżki albo `..`. Nie powinno się to nigdy zdarzyć przy
    /// normalnym użyciu (Hacker Mode zawsze przekazuje tu `id` wprost z
    /// [`list_installed_games`], czyli surową nazwę katalogu bez żadnej
    /// modyfikacji), ale to jedyna linia obrony przed przypadkowym `rm -rf`
    /// gdzie indziej niż w [`GAMES_RELATIVE_DIR`], gdyby `game_id`
    /// kiedykolwiek trafił tu z innego, mniej zaufanego źródła.
    #[error("nieprawidłowy identyfikator gry: `{0}`")]
    InvalidGameId(String),
    /// Zwracane przez [`resolve_game_dir`], gdy katalog o danym `game_id`
    /// nie istnieje w [`GAMES_RELATIVE_DIR`] tego prefiksu.
    #[error("nie znaleziono katalogu gry `{0}` w `{1}`")]
    GameDirNotFound(String, PathBuf),
}

/// Czy `wine` jest w ogóle dostępny w systemie — warunek konieczny (ale
/// niewystarczający, patrz [`find_prefix`]) do jakiejkolwiek integracji z EA app.
pub fn is_available() -> bool {
    which::which("wine").is_ok()
}

/// Szuka prefiksu Wine z zainstalowanym EA app. Kolejność sprawdzania:
/// 1. `explicit` (jeśli podane — np. z ustawień Hacker Mode, dla
///    użytkowników, którzy wybrali nietypową lokalizację).
/// 2. [`KNOWN_PREFIX_CANDIDATES`] względem katalogu domowego.
///
/// Prefiks uznajemy za trafiony, jeśli którakolwiek ze ścieżek
/// [`LAUNCHER_RELATIVE_PATHS`] istnieje wewnątrz niego jako plik.
pub fn find_prefix(explicit: Option<&Path>) -> Result<PathBuf, EaError> {
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
        let hit = LAUNCHER_RELATIVE_PATHS
            .iter()
            .any(|rel| candidate.join(rel).is_file());
        if hit {
            return Ok(candidate.clone());
        }
    }

    Err(EaError::PrefixNotFound(candidates))
}

/// Heurystyczna lista zainstalowanych gier EA — po jednym wpisie na
/// podkatalog w [`GAMES_RELATIVE_DIR`]. Bez dostępu do API EA nie mamy
/// prawdziwych tytułów/okładek/ID produktów — nazwa katalogu instalacji to
/// zarazem `id` i `title`.
pub fn list_installed_games(prefix: &Path) -> Result<Vec<InstalledGame>, EaError> {
    let games_dir = prefix.join(GAMES_RELATIVE_DIR);
    if !games_dir.is_dir() {
        // EA app jest zainstalowany, ale nie ma jeszcze żadnych gier w
        // standardowej lokalizacji (albo użytkownik zainstalował je gdzie
        // indziej ręcznie) — to nie błąd, po prostu pusta biblioteka.
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&games_dir)
        .map_err(|e| EaError::ReadDirFailed(games_dir.clone(), e.to_string()))?;

    let mut games = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            games.push(InstalledGame {
                id: name.clone(),
                title: name,
                install_dir: entry.path(),
            });
        }
    }
    games.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(games)
}

/// Rozwiązuje `game_id` (nazwa katalogu, patrz [`InstalledGame::id`]) do
/// pełnej ścieżki katalogu instalacji tej gry wewnątrz [`GAMES_RELATIVE_DIR`]
/// danego prefiksu, z walidacją chroniącą przed path traversal (patrz
/// [`EaError::InvalidGameId`]) — używane przez `commands::stores::ea`
/// przy budowaniu polecenia odinstalowania (patrz jej moduł-dokumentacja
/// po wyjaśnienie, DLACZEGO to w ogóle jest "odinstalowanie" — bez
/// dostępu do prywatnego API EA to jedyna uczciwa opcja: usunięcie
/// katalogu, w którym EA Desktop trzyma pliki tej gry).
pub fn resolve_game_dir(prefix: &Path, game_id: &str) -> Result<PathBuf, EaError> {
    if game_id.is_empty() || game_id.contains('/') || game_id.contains('\\') || game_id.contains("..") {
        return Err(EaError::InvalidGameId(game_id.to_string()));
    }
    let dir = prefix.join(GAMES_RELATIVE_DIR).join(game_id);
    if !dir.is_dir() {
        return Err(EaError::GameDirNotFound(game_id.to_string(), dir));
    }
    Ok(dir)
}

/// Buduje komendę uruchamiającą sam EA app wewnątrz jego prefiksu Wine
/// (patrz dokumentacja modułu — uruchomienie konkretnej gry z pominięciem
/// EA app nie jest tu obsługiwane, bo wymagałoby nieznanego prywatnego API).
pub fn build_launch_ea_app_command(prefix: &Path) -> Result<Command, EaError> {
    if !is_available() {
        return Err(EaError::WineNotInstalled);
    }
    let launcher = LAUNCHER_RELATIVE_PATHS
        .iter()
        .map(|rel| prefix.join(rel))
        .find(|p| p.is_file())
        .ok_or_else(|| EaError::PrefixNotFound(vec![prefix.to_path_buf()]))?;

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
        let launcher_dir = dir.path().join(
            "drive_c/Program Files/Electronic Arts/EA Desktop/EA Desktop",
        );
        fs::create_dir_all(&launcher_dir).unwrap();
        fs::write(launcher_dir.join("EALauncher.exe"), b"").unwrap();
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
        let err = find_prefix(Some(dir.path()));
        assert!(err.is_err());
    }

    #[test]
    fn lists_game_subdirectories() {
        let dir = fake_prefix();
        let games_dir = dir.path().join(GAMES_RELATIVE_DIR);
        fs::create_dir_all(games_dir.join("Battlefield 2042")).unwrap();
        fs::create_dir_all(games_dir.join("Apex Legends")).unwrap();

        let games = list_installed_games(dir.path()).unwrap();
        let titles: Vec<_> = games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, vec!["Apex Legends", "Battlefield 2042"]);
    }

    #[test]
    fn empty_when_games_dir_missing() {
        let dir = fake_prefix();
        let games = list_installed_games(dir.path()).unwrap();
        assert!(games.is_empty());
    }

    #[test]
    fn resolve_game_dir_finds_existing_game() {
        let dir = fake_prefix();
        let games_dir = dir.path().join(GAMES_RELATIVE_DIR);
        fs::create_dir_all(games_dir.join("Apex Legends")).unwrap();

        let resolved = resolve_game_dir(dir.path(), "Apex Legends").unwrap();
        assert_eq!(resolved, games_dir.join("Apex Legends"));
    }

    #[test]
    fn resolve_game_dir_rejects_path_traversal() {
        let dir = fake_prefix();
        assert!(matches!(
            resolve_game_dir(dir.path(), "../../etc"),
            Err(EaError::InvalidGameId(_))
        ));
        assert!(matches!(
            resolve_game_dir(dir.path(), "sub/dir"),
            Err(EaError::InvalidGameId(_))
        ));
    }

    #[test]
    fn resolve_game_dir_errors_when_not_found() {
        let dir = fake_prefix();
        assert!(matches!(
            resolve_game_dir(dir.path(), "Nonexistent Game"),
            Err(EaError::GameDirNotFound(_, _))
        ));
    }
}
