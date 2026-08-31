use std::path::{Path, PathBuf};
use std::process::Command;

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn game_backup_dir(backup_root: &Path, platform_slug: &str, game_id: &str) -> PathBuf {
    backup_root.join(format!("{platform_slug}-{game_id}"))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupEntry {
    /// Nazwa pliku (`<timestamp>.tar.gz`) — to, co trzeba przekazać z
    /// powrotem do `restore` jako `backup_file_name`.
    pub file_name: String,
    pub created_at: u64,
    pub size_bytes: u64,
}

/// Tworzy nową kopię zapasową katalogu zapisu `save_path` dla danej gry.
/// Zwraca błąd, jeśli `save_path` nie istnieje/nie jest katalogiem — to
/// zwykle oznacza literówkę w ręcznie wpisanej ścieżce (patrz
/// moduł-dokumentacja: świadomie nie zgadujemy tej ścieżki), więc lepiej
/// zgłosić to jasno niż utworzyć puste, mylące archiwum.
pub fn backup_now(backup_root: &Path, platform_slug: &str, game_id: &str, save_path: &str) -> Result<BackupEntry, String> {
    let save_path = Path::new(save_path);
    if !save_path.is_dir() {
        return Err(format!("Katalog zapisu nie istnieje: {}", save_path.display()));
    }
    let Some(parent) = save_path.parent() else {
        return Err(format!("Nie udało się ustalić katalogu nadrzędnego dla {}", save_path.display()));
    };
    let Some(dir_name) = save_path.file_name().and_then(|n| n.to_str()) else {
        return Err("Nieprawidłowa nazwa katalogu zapisu".to_string());
    };

    let dest_dir = game_backup_dir(backup_root, platform_slug, game_id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Nie udało się utworzyć katalogu kopii zapasowych: {e}"))?;

    let timestamp = now_unix();
    let file_name = format!("{timestamp}.tar.gz");
    let dest_path = dest_dir.join(&file_name);

    // `-C parent dir_name` zamiast `tar -czf dest save_path` wprost —
    // dzięki temu archiwum zawiera ścieżki względne zaczynające się od
    // nazwy katalogu zapisu (np. `SaveGames/slot1.sav`), nie absolutne
    // (`/home/user/.../SaveGames/slot1.sav`) — to samo archiwum da się
    // wtedy bezpiecznie rozpakować w INNYM miejscu przy `restore`, gdyby
    // ścieżka zapisu się kiedyś zmieniła.
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(&dest_path)
        .arg("-C")
        .arg(parent)
        .arg(dir_name)
        .status()
        .map_err(|e| format!("Nie udało się uruchomić `tar`: {e}"))?;
    if !status.success() {
        return Err(format!("`tar` zakończył się błędem (kod {:?})", status.code()));
    }

    let size_bytes = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
    prune_old_backups(&dest_dir);
    Ok(BackupEntry { file_name, created_at: timestamp, size_bytes })
}

/// Maksymalna liczba kopii zapasowych trzymanych PER gra — bez limitu
/// automatyczne kopie (patrz `launcher.rs`, backup przy każdym zamknięciu
/// gry, jeśli skonfigurowana) narosłyby w nieskończoność przy częstym
/// graniu. 20 daje sensowną historię (np. dzienne kopie z ostatniego
/// miesiąca grania) bez nieograniczonego zużycia miejsca na dysku —
/// wyraźnie więcej niż limit `compat_tools::MAX_BACKUPS_PER_FILE` (10),
/// bo utrata postępu w grze jest dotkliwsza niż utrata starej wersji
/// ustawienia Proton/Wine.
const MAX_BACKUPS_PER_GAME: usize = 20;

fn prune_old_backups(dest_dir: &Path) {
    let mut backups = list_backups_in_dir(dest_dir);
    if backups.len() <= MAX_BACKUPS_PER_GAME {
        return;
    }
    // `list_backups`/`list_backups_in_dir` sortują od najnowszej do
    // najstarszej (patrz ich dokumentacja) — więc nadmiar do usunięcia to
    // OGON tej listy.
    backups.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let to_remove = backups.len() - MAX_BACKUPS_PER_GAME;
    for entry in backups.into_iter().take(to_remove) {
        let path = dest_dir.join(&entry.file_name);
        if let Err(err) = std::fs::remove_file(&path) {
            tracing::warn!(?err, path = %path.display(), "Nie udało się usunąć przeterminowanej kopii zapasowej zapisu");
        } else {
            tracing::debug!(path = %path.display(), "Usunięto przeterminowaną kopię zapasową zapisu (rotacja)");
        }
    }
}

/// Lista kopii zapasowych dla danej gry, od najnowszej do najstarszej.
pub fn list_backups(backup_root: &Path, platform_slug: &str, game_id: &str) -> Vec<BackupEntry> {
    list_backups_in_dir(&game_backup_dir(backup_root, platform_slug, game_id))
}

fn list_backups_in_dir(dir: &Path) -> Vec<BackupEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut backups: Vec<BackupEntry> = entries
        .flatten()
        .filter_map(|e| {
            let file_name = e.file_name().to_string_lossy().into_owned();
            let timestamp_str = file_name.strip_suffix(".tar.gz")?;
            let created_at: u64 = timestamp_str.parse().ok()?;
            let size_bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            Some(BackupEntry { file_name, created_at, size_bytes })
        })
        .collect();
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    backups
}

/// Przywraca kopię zapasową `backup_file_name` do `save_path`, NADPISUJĄC
/// jego aktualną zawartość. Przed nadpisaniem tworzy WŁASNĄ kopię
/// zapasową aktualnego stanu (`safety_backup_now`, patrz niżej) — jeśli
/// przywrócenie okaże się pomyłką (np. przywrócono za starą kopię),
/// poprzedni stan i tak zostaje na dysku w tym samym miejscu co reszta
/// kopii, gotowy do ponownego przywrócenia.
pub fn restore_backup(backup_root: &Path, platform_slug: &str, game_id: &str, backup_file_name: &str, save_path: &str) -> Result<(), String> {
    // Walidacja nazwy pliku przeciw path traversal — `backup_file_name`
    // przychodzi z frontendu (przez `commands::restore_game_save_backup`),
    // więc jest to jedyna linia obrony przed np. `"../../../etc/passwd"`.
    if backup_file_name.contains('/') || backup_file_name.contains('\\') || backup_file_name.contains("..") {
        return Err(format!("Nieprawidłowa nazwa pliku kopii zapasowej: {backup_file_name}"));
    }

    let backup_path = game_backup_dir(backup_root, platform_slug, game_id).join(backup_file_name);
    if !backup_path.is_file() {
        return Err(format!("Nie znaleziono kopii zapasowej: {}", backup_path.display()));
    }

    let save_path_buf = Path::new(save_path);
    let parent = save_path_buf.parent().ok_or("Nieprawidłowa ścieżka zapisu")?;
    // Kopia bezpieczeństwa AKTUALNEGO stanu przed nadpisaniem — patrz
    // dokumentacja funkcji. Błąd tworzenia tej kopii NIE blokuje
    // przywracania (użytkownik świadomie klika "Przywróć", wie, co robi),
    // ale jest głośno logowany.
    if save_path_buf.is_dir() {
        if let Err(err) = backup_now(backup_root, platform_slug, game_id, save_path) {
            tracing::warn!(%err, "Nie udało się utworzyć kopii bezpieczeństwa przed przywróceniem");
        }
    }

    std::fs::create_dir_all(parent).map_err(|e| format!("Nie udało się przygotować katalogu docelowego: {e}"))?;

    // BUGFIX (zgłoszone przez `cargo test` w CI — `logs_Hacker-Mode.zip`,
    // test `backup_and_restore_roundtrip`): wcześniejsza wersja łączyła
    // `-C parent` z `--strip-components=1`, co wypakowywało pliki o JEDEN
    // POZIOM ZA WYSOKO. Archiwum ma w środku jeden katalog najwyższego
    // poziomu o nazwie DOKŁADNIE takiej jak `save_path` (patrz
    // `backup_now`: `tar -C parent dir_name`, gdzie `dir_name` to nazwa
    // katalogu zapisu) — więc rozpakowanie go z `-C parent` BEZ ścinania
    // komponentów samo z siebie odtwarza `parent/<dir_name>/...`, czyli
    // dokładnie `save_path`. Poprzednia wersja dodatkowo ŚCINAŁA ten sam
    // komponent, co w połączeniu z `-C parent` (zamiast `-C save_path`)
    // wypakowywało zawartość PROSTO DO `parent`, pomijając katalog
    // `save_path` w ogóle — plik, którego test szukał pod
    // `save_path/slot1.sav`, faktycznie lądował pod `parent/slot1.sav`.
    // Nie potrzeba już osobnego `create_dir_all(save_path)` — `tar`
    // odtworzy ten katalog sam, z samej struktury archiwum.
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&backup_path)
        .arg("-C")
        .arg(parent)
        .status()
        .map_err(|e| format!("Nie udało się uruchomić `tar`: {e}"))?;
    if !status.success() {
        return Err(format!("`tar` zakończył się błędem przy rozpakowywaniu (kod {:?})", status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_available(name: &str) -> bool {
        std::process::Command::new("which").arg(name).output().map(|o| o.status.success()).unwrap_or(false)
    }

    #[test]
    fn game_backup_dir_uses_platform_and_id() {
        let root = Path::new("/tmp/backups");
        assert_eq!(game_backup_dir(root, "epic", "abc"), Path::new("/tmp/backups/epic-abc"));
    }

    #[test]
    fn prune_old_backups_keeps_only_newest_within_limit() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..(MAX_BACKUPS_PER_GAME + 5) {
            std::fs::write(dir.path().join(format!("{i}.tar.gz")), b"x").unwrap();
        }
        prune_old_backups(dir.path());
        let remaining = list_backups_in_dir(dir.path());
        assert_eq!(remaining.len(), MAX_BACKUPS_PER_GAME);
        // Zostają najwyższe (najnowsze) znaczniki czasu.
        assert!(remaining.iter().all(|b| b.created_at >= 5));
    }

    #[test]
    fn list_backups_missing_directory_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_backups(dir.path(), "epic", "nonexistent").is_empty());
    }

    #[test]
    fn restore_backup_rejects_path_traversal_in_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let result = restore_backup(dir.path(), "epic", "abc", "../../etc/passwd", "/tmp/wherever");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Nieprawidłowa nazwa"));
    }

    #[test]
    fn backup_now_rejects_missing_save_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = backup_now(dir.path(), "epic", "abc", "/nieistniejacy/katalog/zapisu");
        assert!(result.is_err());
    }

    // Ten test faktycznie tworzy i rozpakowuje archiwum `tar` — jedyny
    // test w projekcie wołający zewnętrzny binarny program (`tar` jest
    // praktycznie wszechobecny na Linuksie, w przeciwieństwie do
    // `steam`/`legendary`/itp., więc to bezpieczne założenie w CI). Jeśli
    // `tar` nie jest dostępne w środowisku testowym, test jest pomijany
    // zamiast fałszywie failować.
    #[test]
    fn backup_and_restore_roundtrip() {
        if !tool_available("tar") {
            eprintln!("pomijam: `tar` niedostępne w tym środowisku");
            return;
        }
        let backup_root = tempfile::tempdir().unwrap();
        let save_dir = tempfile::tempdir().unwrap();
        let save_path = save_dir.path().join("MyGameSave");
        std::fs::create_dir_all(&save_path).unwrap();
        std::fs::write(save_path.join("slot1.sav"), b"dane zapisu gry").unwrap();

        let entry = backup_now(backup_root.path(), "epic", "abc", save_path.to_str().unwrap()).unwrap();
        assert!(entry.size_bytes > 0);

        let backups = list_backups(backup_root.path(), "epic", "abc");
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].file_name, entry.file_name);

        // Symulujemy utratę zapisu (np. przypadkowe usunięcie przez grę).
        std::fs::remove_dir_all(&save_path).unwrap();
        assert!(!save_path.exists());

        restore_backup(backup_root.path(), "epic", "abc", &entry.file_name, save_path.to_str().unwrap()).unwrap();

        let restored = std::fs::read_to_string(save_path.join("slot1.sav")).unwrap();
        assert_eq!(restored, "dane zapisu gry");
    }
}
