use super::{tool_available, Game, Platform, StoreError, StoreProvider};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
pub struct LutrisProvider;

/// Ścieżka do bazy SQLite, w której Lutris trzyma całą swoją bibliotekę
/// (status instalacji, katalog gry, ZMIERZONY PRZEZ SAM LUTRIS czas gry w
/// polu `playtime`) — udokumentowana wprost we własnym README Lutrisa:
/// "pga.db: An SQLite database tracking the game library, game
/// installation status, various file locations, and some additional
/// metadata" (<https://github.com/lutris/lutris>, sekcja "Files"). Kolumny
/// `slug`/`installed`/`directory`/`playtime` potwierdzone niezależnie w
/// źródle samego Lutrisa (`lutris/database/games.py`, filtrowanie po
/// `installed`/`slug`) oraz w wątkach na forum Lutrisa opisujących ręczną
/// edycję `playtime` (wartość w GODZINACH jako liczba zmiennoprzecinkowa,
/// stąd `* 60.0` przy konwersji na minuty w [`fetch_playtime_minutes`]).
pub(crate) fn pga_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join("lutris")
        .join("pga.db")
}

/// Ucieka pojedynczy cudzysłów dla bezpiecznego osadzenia wartości w
/// zapytaniu SQL przekazywanym przez CLI `sqlite3` (nie mamy tu
/// prawdziwych parametrów bindowanych — CLI `sqlite3` nie oferuje ich przez
/// proste argumenty wiersza poleceń, tylko przez tryb interaktywny/`.read`,
/// którego nie warto tu uruchamiać dla jednego zapytania). `slug` pochodzi
/// zawsze z własnego JSON-a Lutrisa (`lutris -l --json`, patrz
/// `list_games`), więc ryzyko złośliwej wartości jest w praktyce zerowe —
/// ale i tak nie ma powodu, żeby nie zrobić tego poprawnie.
fn sql_escape(value: &str) -> String {
    value.replace('\'', "''")
}

impl StoreProvider for LutrisProvider {
    fn platform(&self) -> Platform {
        Platform::Lutris
    }

    fn is_available(&self) -> bool {
        tool_available("lutris")
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let output = Command::new("lutris")
            .args(["-l", "--json"])
            .output()
            .map_err(|e| StoreError::ExecFailed("lutris".into(), e.to_string()))?;

        if !output.status.success() {
            return Err(StoreError::ExecFailed(
                "lutris".into(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(&stdout)
            .map_err(|e| StoreError::ParseError("lutris -l --json".into(), e.to_string()))?;

        // Czas gry: `lutris -l --json` (wywołanie wyżej) NIE zwraca pola
        // `playtime` — jedyne miejsce, gdzie Lutris go trzyma, to kolumna
        // `playtime` w jego własnej bazie `pga.db` (patrz `pga_db_path`).
        // Jedno zbiorcze zapytanie zamiast pytania bazy osobno dla każdej
        // gry — patrz `fetch_playtime_minutes`.
        let playtime_by_slug = fetch_playtime_minutes();

        let entries = parsed.as_array().cloned().unwrap_or_default();
        let games = entries
            .into_iter()
            .filter_map(|entry| {
                let slug = entry.get("slug")?.as_str()?.to_string();
                let title = entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&slug)
                    .to_string();
                let directory = entry
                    .get("directory")
                    .and_then(Value::as_str)
                    .map(String::from);
                let playtime_minutes = playtime_by_slug.get(&slug).copied();

                Some(Game {
                    cover_path: find_local_cover_art(&slug),
                    id: slug,
                    title,
                    platform: Platform::Lutris,
                    installed: true,
                    install_dir: directory,
                    playtime_minutes,
                })
            })
            .collect();

        Ok(games)
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // Lutris obsługuje protokół `lutris:` do wyzwalania własnych
        // instalatorów gier (pobranych z lutris.net) — jeśli gra o danym
        // slugu ma zdefiniowany instalator, otworzy się kreator instalacji
        // w oknie Lutrisa.
        let mut cmd = Command::new("lutris");
        cmd.arg(format!("lutris:{game_id}"));
        Ok(cmd)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // Lutris NIE ma stabilnego, publicznie udokumentowanego przełącznika
        // CLI do odinstalowania pojedynczej gry (próba dodania takiego
        // przełącznika — `lutris --uninstall id/slug` — istniała w PR
        // lutris/lutris#3029, ale nie weszła do głównej gałęzi; dzisiejszy
        // oficjalny `README.rst` Lutrisa listuje `-u/--uninstall-runner`,
        // czyli usuwanie RUNNERA, nie gry). Zamiast zgadywać nieistniejący
        // przełącznik, robimy dokładnie to, co robi własne okno Lutrisa
        // przy odznaczeniu gry z biblioteki (bez usuwania plików — patrz
        // `lutris/gui/dialogs/uninstall_dialog.py`, opcja "Remove from
        // library" bez "Delete files"): oznaczamy grę jako
        // NIEZAINSTALOWANĄ bezpośrednio w `pga.db` przez CLI `sqlite3`.
        // To NIE usuwa plików gry z dysku (bezpieczniejszy domyślny wybór
        // niż zgadywanie ścieżki i `rm -rf` na cudzej bazie danych) — tylko
        // czyści status w bibliotece Lutrisa, dokładnie jak "Remove from
        // library". Użytkownik może doczyścić pliki ręcznie, jeśli chce.
        if !tool_available("sqlite3") {
            return Err(StoreError::ToolNotInstalled("sqlite3".into()));
        }
        let db_path = pga_db_path();
        if !db_path.is_file() {
            return Err(StoreError::ToolNotInstalled(format!(
                "baza Lutrisa ({})",
                db_path.display()
            )));
        }
        // Opcjonalne kasowanie plików (Ustawienia → `lutris_delete_files_on_uninstall`,
        // domyślnie WYŁĄCZONE). Katalog gry musimy odczytać TERAZ, bo poniższe
        // UPDATE czyści kolumnę `directory`; faktyczne kasowanie następuje
        // dopiero po potwierdzonym sukcesie UPDATE-a (`on_uninstall_finished`).
        if crate::settings::Settings::load_or_default().lutris_delete_files_on_uninstall {
            if let Some(dir) = directory_for_slug(game_id) {
                pending_deletions().lock().unwrap_or_else(|e| e.into_inner()).insert(game_id.to_string(), dir);
            }
        }
        let sql = format!(
            "UPDATE games SET installed = 0, directory = NULL WHERE slug = '{}';",
            sql_escape(game_id)
        );
        let mut cmd = Command::new("sqlite3");
        cmd.arg(db_path);
        cmd.arg(sql);
        Ok(cmd)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        let mut cmd = Command::new("lutris");
        cmd.arg(format!("rungame/{game_id}"));
        Ok(cmd)
    }
}

/// Wczytuje czas gry (w minutach) dla WSZYSTKICH gier w `pga.db` naraz,
/// jednym zapytaniem przez CLI `sqlite3` — zwraca pustą mapę (nie błąd),
/// jeśli `sqlite3` nie jest zainstalowane albo baza nie istnieje, żeby
/// brak tej JEDNEJ funkcji nigdy nie blokował wyświetlenia reszty
/// biblioteki Lutrisa. `-noheader -separator` wymusza stabilny, prosty do
/// sparsowania format `slug|godziny` — bez tego domyślny separator
/// `sqlite3` też jest `|`, ale jawne ustawienie chroni przed niespodzianką,
/// gdyby użytkownik miał globalnie inaczej skonfigurowany plik `.sqliterc`.
fn fetch_playtime_minutes() -> std::collections::HashMap<String, u64> {
    let mut result = std::collections::HashMap::new();
    if !tool_available("sqlite3") {
        return result;
    }
    let db_path = pga_db_path();
    if !db_path.is_file() {
        return result;
    }

    let output = Command::new("sqlite3")
        .args(["-noheader", "-separator", "|"])
        .arg(&db_path)
        .arg("SELECT slug, playtime FROM games WHERE playtime IS NOT NULL AND playtime > 0;")
        .output();

    let Ok(output) = output else {
        return result;
    };
    if !output.status.success() {
        tracing::debug!(
            stderr = %String::from_utf8_lossy(&output.stderr),
            "Lutris: nie udało się odczytać czasu gry z pga.db"
        );
        return result;
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((slug, hours_str)) = line.split_once('|') else {
            continue;
        };
        if let Ok(hours) = hours_str.trim().parse::<f64>() {
            let minutes = (hours * 60.0).round().max(0.0) as u64;
            result.insert(slug.to_string(), minutes);
        }
    }
    result
}

/// Szuka lokalnie już pobranego przez sam Lutris bannera/coverartu dla
/// gry o danym slugu — Lutris pobiera je sam (z IGDB) przy pierwszym
/// dodaniu gry, więc w przeciwieństwie do Epic/GOG/Amazon nie trzeba tu
/// nic ściągać z sieci, tylko wskazać istniejący plik.
///
/// Lokalizacje i konwencja nazewnictwa (`<slug>.jpg`) potwierdzone przez
/// dokumentację/issues samego projektu Lutris:
/// - `~/.local/share/lutris/{coverart,banners}/` — aktualna lokalizacja
///   (od wersji 0.5.18, patrz
///   <https://github.com/lutris/lutris/pull/5897>).
/// - `~/.cache/lutris/{coverart,banners}/` — starsza lokalizacja, wciąż
///   używana przez część zainstalowanych wersji (patrz
///   <https://github.com/lutris/lutris/issues/5882>).
///
/// Sprawdzamy coverart przed bannerem (bardziej pionowy, bliższy
/// proporcjom okładek innych platform w siatce biblioteki Hacker Mode),
/// obie lokalizacje, obie popularne rozszerzenia — pierwsze trafienie wygrywa.
fn find_local_cover_art(slug: &str) -> Option<String> {
    let candidates: [Option<std::path::PathBuf>; 2] = [dirs::data_dir(), dirs::cache_dir()];

    for base in candidates.into_iter().flatten() {
        for subdir in ["coverart", "banners"] {
            for ext in ["jpg", "jpeg", "png"] {
                let path = base.join("lutris").join(subdir).join(format!("{slug}.{ext}"));
                if path.is_file() {
                    return Some(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------
// Opcjonalne kasowanie plików gry po odinstalowaniu
// ---------------------------------------------------------------------

fn pending_deletions() -> &'static Mutex<HashMap<String, PathBuf>> {
    static PENDING: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Katalog instalacji gry zapisany w `pga.db` (kolumna `directory`).
fn directory_for_slug(slug: &str) -> Option<PathBuf> {
    if !tool_available("sqlite3") {
        return None;
    }
    let db = pga_db_path();
    if !db.is_file() {
        return None;
    }
    let sql = format!("SELECT directory FROM games WHERE slug = '{}' LIMIT 1;", sql_escape(slug));
    let out = Command::new("sqlite3").arg("-noheader").arg(&db).arg(sql).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() || value == "NULL" {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

/// Katalogi, z których wolno kasować pliki gier Lutrisa. Świadomie wąska
/// lista: domyślny `~/Games`, dane Lutrisa i wersja Flatpak. Gra zainstalowana
/// gdzie indziej zostanie tylko usunięta z biblioteki (pliki zostają).
fn allowed_delete_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join("Games"),
        dirs::data_dir().unwrap_or_default().join("lutris"),
        home.join(".var/app/net.lutris.Lutris/data/lutris"),
    ]
}

/// Czy `dir` można bezpiecznie skasować: musi istnieć, nie być dowiązaniem
/// symbolicznym i leżeć STRICTE POD jednym z `roots` (nigdy sam root -
/// `~/Games` jako całość nie zostanie skasowany). Porównanie po
/// `canonicalize`, więc `..` i symlinki pośrednie nie pomogą.
pub(crate) fn is_safe_to_delete(dir: &Path, roots: &[PathBuf]) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(dir) else { return false };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return false;
    }
    let Ok(canonical) = dir.canonicalize() else { return false };
    roots.iter().filter_map(|r| r.canonicalize().ok()).any(|root| canonical != root && canonical.starts_with(&root))
}

/// Wywoływane po potwierdzonym sukcesie odinstalowania (patrz
/// `stores::on_uninstall_finished`).
pub(crate) fn finish_uninstall(slug: &str) {
    let Some(dir) = pending_deletions().lock().unwrap_or_else(|e| e.into_inner()).remove(slug) else {
        return;
    };
    if !is_safe_to_delete(&dir, &allowed_delete_roots()) {
        tracing::warn!(dir = %dir.display(), "Pomijam kasowanie plików gry Lutrisa - katalog poza dozwolonymi lokalizacjami (gra tylko usunięta z biblioteki)");
        return;
    }
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => tracing::info!(dir = %dir.display(), "Skasowano pliki gry Lutrisa"),
        Err(e) => tracing::warn!(dir = %dir.display(), error = %e, "Nie udało się skasować plików gry Lutrisa"),
    }
}

#[cfg(test)]
mod delete_tests {
    use super::*;

    #[test]
    fn only_strictly_inside_allowed_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Games");
        let game = root.join("witcher3");
        let outside = tmp.path().join("Documents");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let roots = vec![root.clone()];

        assert!(is_safe_to_delete(&game, &roots));
        assert!(!is_safe_to_delete(&root, &roots), "sam root nigdy");
        assert!(!is_safe_to_delete(&outside, &roots));
        assert!(!is_safe_to_delete(&root.join("nie-istnieje"), &roots));
    }

    #[test]
    fn symlinks_and_dotdot_escapes_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Games");
        let outside = tmp.path().join("Documents");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let roots = vec![root.clone()];

        assert!(!is_safe_to_delete(&link, &roots), "symlink");
        assert!(!is_safe_to_delete(&root.join("..").join("Documents"), &roots), "ucieczka przez ..");
    }
}
