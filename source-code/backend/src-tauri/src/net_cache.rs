use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    fetched_at: u64,
    payload: serde_json::Value,
}

fn cache_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("hacker-mode/cache")
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load_file(path: &Path) -> HashMap<String, CacheEntry> {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

fn save_file(path: &Path, data: &HashMap<String, CacheEntry>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(data) {
        let _ = std::fs::write(path, json);
    }
}

/// Zwraca wartość z cache'a pod `key` w pliku `file_name`, jeśli istnieje
/// i jest młodsza niż `ttl_seconds` — inaczej `None` (traktowane przez
/// wywołującego jako "trzeba dociągnąć na nowo", patrz
/// `get_or_fetch` niżej, który jest zwykle wygodniejszym punktem wejścia
/// niż wołanie `get`/`set` osobno).
pub fn get<T: DeserializeOwned>(file_name: &str, key: &str, ttl_seconds: u64) -> Option<T> {
    let data = load_file(&cache_dir().join(file_name));
    let entry = data.get(key)?;
    if now_unix().saturating_sub(entry.fetched_at) > ttl_seconds {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

pub fn set<T: Serialize>(file_name: &str, key: &str, value: &T) {
    let path = cache_dir().join(file_name);
    let mut data = load_file(&path);
    let Ok(payload) = serde_json::to_value(value) else {
        return;
    };
    data.insert(key.to_string(), CacheEntry { fetched_at: now_unix(), payload });
    save_file(&path, &data);
}

/// Wygodny punkt wejścia łączący `get`+`fetch`+`set`: sprawdza cache,
/// a przy braku/przeterminowaniu woła `fetch` i zapisuje wynik (nawet gdy
/// `fetch` zwróci `None` — patrz niżej, dlaczego to ZAMIERZONE).
///
/// `fetch` zwracające `None` JEST cache'owane tak samo jak `Some` (przez
/// `Option<T>` samo w sobie będące `T` dla `serde`) — bez tego każde
/// wejście w szczegóły gry BEZ oceny ProtonDB (bo np. nowa/niszowa gra
/// nie ma jeszcze zgłoszeń) odpytywałoby `protondb.com` od nowa za każdym
/// razem, co jest dokładnie tym ruchem sieciowym, którego ten moduł ma
/// unikać — "brak wyniku" to też wynik wart zapamiętania na czas TTL.
pub fn get_or_fetch<T, F>(file_name: &str, key: &str, ttl_seconds: u64, fetch: F) -> Option<T>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Option<T>,
{
    if let Some(cached) = get::<Option<T>>(file_name, key, ttl_seconds) {
        return cached;
    }
    let fresh = fetch();
    set(file_name, key, &fresh);
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    struct Sample {
        value: u32,
    }

    fn with_temp_cache_dir<T>(f: impl FnOnce(&Path) -> T) -> T {
        let dir = tempfile::tempdir().unwrap();
        f(dir.path())
    }

    // Te testy wołają `load_file`/`save_file` bezpośrednio (nie
    // `get`/`set`, które są zahardkodowane na `cache_dir()`), żeby nie
    // dotykać prawdziwego `~/.local/share/hacker-mode/cache/` na maszynie,
    // na której akurat lecą testy — ten sam kompromis co w `playtime.rs`
    // (tam jest to rozwiązane przez `load_from`/`save_to` przyjmujące
    // ścieżkę, tu robimy dokładnie to samo, tylko na poziomie tego testu
    // zamiast zmieniać publiczne API modułu).
    #[test]
    fn save_and_load_roundtrip() {
        with_temp_cache_dir(|dir| {
            let path = dir.join("test.json");
            let mut data = HashMap::new();
            data.insert(
                "440".to_string(),
                CacheEntry { fetched_at: now_unix(), payload: serde_json::to_value(Sample { value: 42 }).unwrap() },
            );
            save_file(&path, &data);

            let loaded = load_file(&path);
            let entry = loaded.get("440").unwrap();
            let sample: Sample = serde_json::from_value(entry.payload.clone()).unwrap();
            assert_eq!(sample, Sample { value: 42 });
        });
    }

    #[test]
    fn load_missing_file_returns_empty_map() {
        with_temp_cache_dir(|dir| {
            let loaded = load_file(&dir.join("does-not-exist.json"));
            assert!(loaded.is_empty());
        });
    }

    #[test]
    fn expired_entry_is_treated_as_stale() {
        with_temp_cache_dir(|dir| {
            let path = dir.join("test.json");
            let mut data = HashMap::new();
            // Wpis "z przeszłości", starszy niż jakikolwiek rozsądny TTL.
            data.insert(
                "440".to_string(),
                CacheEntry { fetched_at: 0, payload: serde_json::to_value(Sample { value: 1 }).unwrap() },
            );
            save_file(&path, &data);

            let loaded = load_file(&path);
            let entry = loaded.get("440").unwrap();
            assert!(now_unix().saturating_sub(entry.fetched_at) > 3600);
        });
    }
}
