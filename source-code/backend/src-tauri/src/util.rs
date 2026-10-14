use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// `Mutex::lock().unwrap()` zatruwa CAŁĄ aplikację po jednej panice w
/// dowolnym wątku trzymającym blokadę (każde kolejne `unwrap()` też
/// panikuje - w tym w wątkach czekających na zakończenie gry, które
/// zapisują czas gry i kopie zapasowe). Dane w naszych mutexach
/// (ustawienia, mapy PID-ów) są prostymi wartościami, które nie zostają w
/// niespójnym stanie "w pół operacji", więc bezpieczniej jest odzyskać
/// blokadę (`into_inner`) i zalogować ostrzeżenie niż kaskadowo wywalić
/// powłokę.
pub trait LockExt<T> {
    fn lock_safe(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_safe(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("Odzyskano zatruty mutex (wcześniejsza panika w innym wątku)");
            poisoned.into_inner()
        })
    }
}

/// Zapisuje plik atomowo: dane trafiają do pliku tymczasowego w TYM SAMYM
/// katalogu (`fsync`), po czym `rename` podmienia cel. Przerwanie zapisu
/// (crash, brak miejsca, zabicie procesu) zostawia albo starą, albo nową
/// kompletną zawartość - nigdy ucięty plik. `mode` (np. `Some(0o600)`)
/// ustawia uprawnienia pliku tymczasowego od momentu utworzenia, więc
/// sekrety nie są nawet na chwilę czytelne dla innych użytkowników.
pub fn write_atomic(path: &Path, data: &[u8], mode: Option<u32>) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));

    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        if let Some(mode) = mode {
            options.mode(mode);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Czy `name` jest dostępne jako polecenie w `PATH` (lub, jeśli zawiera `/`,
/// jako wykonywalny plik).
pub fn command_exists(name: &str) -> bool {
    which::which(name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_safe_recovers_from_poison() {
        let m = std::sync::Arc::new(Mutex::new(5));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("celowa panika");
        })
        .join();
        assert!(m.lock().is_err(), "mutex powinien być zatruty");
        assert_eq!(*m.lock_safe(), 5);
    }

    #[test]
    fn write_atomic_replaces_content_and_sets_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.json");
        write_atomic(&path, b"one", Some(0o600)).unwrap();
        write_atomic(&path, b"two", Some(0o600)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        // brak śmieci po tymczasowym pliku
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
}
