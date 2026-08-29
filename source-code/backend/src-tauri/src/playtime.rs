use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Wartość trzymana per gra. `#[serde(untagged)]` w `RawEntry` (patrz
/// niżej) obsługuje też STARY format pliku sprzed dodania statystyk
/// biblioteki (gdzie wartość to było gołe `u64` — sama liczba minut, bez
/// `last_played_at`) — bez tego każdy plik `playtime.json` zapisany przed
/// tą zmianą przestałby się wczytywać (`load_from` traktowałby go jako
/// uszkodzony i zaczynał od zera, GUBIĄC cały dotychczasowy czas gry
/// wszystkich użytkowników, którzy zdążyli już korzystać z Hacker Mode —
/// nie do zaakceptowania dla czegoś, co i tak nigdy nie zostało
/// przetestowane na żywo, ale to nie powód, żeby robić to niedbale).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Entry {
    minutes: u64,
    /// Sekundy od epoki Unix ostatniego zamknięcia procesu tej gry
    /// (patrz `launcher::run_wrapped`), `0` = nieznane (import ze starego
    /// formatu pliku, patrz `RawEntry`).
    last_played_at: u64,
}

/// Pomocniczy typ WYŁĄCZNIE do (de)serializacji — patrz komentarz przy
/// `Entry` po wyjaśnienie `#[serde(untagged)]`. `serde` próbuje kolejno
/// wariantów w kolejności deklaracji, więc `Full` (obiekt z dwoma polami)
/// musi być pierwszy: liczba nigdy nie sparsuje się jako obiekt, ale dla
/// jasności i bezpiecznej kolejności zachowujemy `Full` przed `Legacy`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Full(Entry),
    Legacy(u64),
}

impl From<RawEntry> for Entry {
    fn from(raw: RawEntry) -> Self {
        match raw {
            RawEntry::Full(e) => e,
            RawEntry::Legacy(minutes) => Entry { minutes, last_played_at: 0 },
        }
    }
}

fn store_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("hacker-mode")
        .join("playtime.json")
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Rdzeń `load()` — wydzielony jako osobna funkcja przyjmująca `path`
/// jawnie (zamiast wołać `store_path()` w środku), żeby dało się go
/// przetestować na tymczasowym pliku (patrz `mod tests`) bez dotykania
/// prawdziwego `~/.local/share/hacker-mode/playtime.json` na maszynie, na
/// której akurat lecą testy.
fn load_from(path: &Path) -> HashMap<String, Entry> {
    let raw: HashMap<String, RawEntry> = match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|err| {
            tracing::warn!(%err, "Nieprawidłowy plik playtime.json, zaczynam od pustego licznika");
            HashMap::new()
        }),
        Err(_) => HashMap::new(),
    };
    raw.into_iter().map(|(k, v)| (k, v.into())).collect()
}

fn load() -> HashMap<String, Entry> {
    load_from(&store_path())
}

/// Rdzeń `save()` — patrz `load_from` po wyjaśnienie, dlaczego `path` jest
/// jawnym parametrem. Zapisuje zawsze w NOWYM formacie (`Entry`, nie
/// `RawEntry::Legacy`) — plik "aktualizuje się" do nowego formatu przy
/// pierwszym zapisie po wczytaniu starego, bez osobnej migracji.
fn save_to(path: &Path, data: &HashMap<String, Entry>) {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(%err, "Nie udało się utworzyć katalogu na playtime.json");
            return;
        }
    }
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Err(err) = std::fs::write(path, json) {
                tracing::warn!(%err, path = %path.display(), "Nie udało się zapisać playtime.json");
            }
        }
        Err(err) => tracing::warn!(%err, "Nie udało się zserializować playtime.json"),
    }
}

fn key(platform_slug: &str, game_id: &str) -> String {
    format!("{platform_slug}:{game_id}")
}

/// Dolicza `minutes` do dotychczasowego czasu gry dla danej pary
/// platforma/gra i ustawia `last_played_at` na TERAZ — wywoływane raz,
/// przy zamknięciu procesu gry (patrz `launcher::run_wrapped`).
/// `minutes` może być `0` (sesje krótsze niż minuta), co jest tu celowo
/// dopuszczalne: nie chcemy zaniżać sumy przez zaokrąglanie w dół przy
/// KAŻDYM pojedynczym wywołaniu (patrz wywołanie w `launcher.rs`, gdzie
/// zaokrąglenie w dół następuje dopiero raz, na sumie sekund tej sesji).
/// `last_played_at` aktualizuje się ZAWSZE (nawet przy `minutes == 0`) —
/// sesja krótsza niż minuta wciąż jest sesją, więc gra wciąż "właśnie
/// była grana", nawet jeśli nie doliczyła nic mierzalnego do sumy czasu.
pub fn add_minutes(platform_slug: &str, game_id: &str, minutes: u64) {
    add_minutes_at(&store_path(), platform_slug, game_id, minutes);
}

fn add_minutes_at(path: &Path, platform_slug: &str, game_id: &str, minutes: u64) {
    let mut data = load_from(path);
    let entry = data.entry(key(platform_slug, game_id)).or_insert(Entry { minutes: 0, last_played_at: 0 });
    entry.minutes = entry.minutes.saturating_add(minutes);
    entry.last_played_at = now_unix();
    save_to(path, &data);
    tracing::debug!(platform_slug, game_id, minutes, "Zapisano lokalny czas gry");
}

/// Zwraca dotychczas zmierzony lokalnie czas gry (w minutach) dla danej
/// pary platforma/gra, jeśli Hacker Mode kiedykolwiek ją uruchomił.
pub fn get_minutes(platform_slug: &str, game_id: &str) -> Option<u64> {
    load().get(&key(platform_slug, game_id)).map(|e| e.minutes)
}

/// Uzupełnia `playtime_minutes` na liście gier lokalnie zmierzonym czasem
/// TYLKO tam, gdzie provider platformy nie zwrócił własnej wartości
/// (`None`) — patrz moduł-dokumentacja: to zapasowe, nie nadrzędne źródło.
/// Wywoływane raz, na końcu `commands::list_games`, po scaleniu wyników
/// wszystkich providerów (i, dla Steam, po dociągnięciu gier posiadanych)
/// — dokładnie tak samo jak `cover_overrides` w tym samym miejscu.
pub fn merge_into(games: &mut [crate::commands::stores::Game]) {
    merge_into_from(&load(), games);
}

/// Rdzeń `merge_into` — przyjmuje już wczytaną mapę zamiast czytać plik
/// samemu, żeby dało się przetestować logikę scalania (patrz `mod tests`)
/// bez dotykania systemu plików w ogóle.
fn merge_into_from(data: &HashMap<String, Entry>, games: &mut [crate::commands::stores::Game]) {
    if data.is_empty() {
        return;
    }
    for game in games.iter_mut() {
        if game.playtime_minutes.is_none() {
            if let Some(entry) = data.get(&key(game.platform.slug(), &game.id)) {
                game.playtime_minutes = Some(entry.minutes);
            }
        }
    }
}

/// Jeden wiersz statystyk biblioteki (patrz `commands::get_library_stats`)
/// — WYŁĄCZNIE dane z tego lokalnego licznika (`minutes`/`last_played_at`),
/// więc dla gier Steam/Lutris z WŁASNYM, dokładniejszym czasem gry (patrz
/// moduł-dokumentacja) statystyki oparte na tym module mogą nie zgadzać
/// się z tym, co pokazuje karta gry w bibliotece (`Game::playtime_minutes`
/// bierze pod uwagę źródło platformy jako nadrzędne, to tutaj — zawsze
/// lokalny licznik). Świadomy kompromis, udokumentowany też we froncie
/// (`Stats.tsx`), bo połączenie DWÓCH źródeł prawdy dla zbiorczych
/// statystyk wymagałoby przechowywania pełnej listy gier tutaj, czego ten
/// moduł celowo nie robi (nie jest cache'em biblioteki, tylko licznikiem
/// czasu).
#[derive(Debug, Clone, Serialize)]
pub struct PlaytimeStatsEntry {
    pub platform_slug: String,
    pub game_id: String,
    pub minutes: u64,
    pub last_played_at: u64,
}

/// Zwraca WSZYSTKIE wpisy lokalnego licznika naraz — używane przez
/// `commands::get_library_stats`, który dopiero łączy to z tytułami/
/// okładkami z aktualnej listy gier (przekazanej z frontendu, patrz jego
/// moduł-dokumentacja) po `platform_slug`+`game_id`.
pub fn all_entries() -> Vec<PlaytimeStatsEntry> {
    load()
        .into_iter()
        .filter_map(|(k, entry)| {
            let (platform_slug, game_id) = k.split_once(':')?;
            Some(PlaytimeStatsEntry {
                platform_slug: platform_slug.to_string(),
                game_id: game_id.to_string(),
                minutes: entry.minutes,
                last_played_at: entry.last_played_at,
            })
        })
        .collect()
}

// ===================== Historia sesji =====================
//
// Osobny plik (`sessions.json`) od `playtime.json` wyżej — celowo:
// `playtime.json` trzyma tylko SUMĘ (i ostatni znacznik czasu), a historia
// to rosnąca LISTA poszczególnych sesji per gra. Trzymanie ich w jednym
// pliku oznaczałoby, że KAŻDY zapis czasu gry (czyli KAŻDE zamknięcie
// JAKIEJKOLWIEK gry) przepisywałby cały, coraz większy plik z historią
// wszystkich gier naraz — osobny plik pozwala `add_minutes`
// (`playtime.json`, mały, stały rozmiar per gra) zostać tanie, podczas
// gdy historia sesji (`sessions.json`, rosnąca, ale ograniczona per gra)
// jest osobną operacją.

const MAX_SESSIONS_PER_GAME: usize = 50;

fn sessions_store_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("hacker-mode")
        .join("sessions.json")
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionEntry {
    pub at: u64,
    pub minutes: u64,
}

fn load_sessions_from(path: &Path) -> HashMap<String, Vec<SessionEntry>> {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

fn save_sessions_to(path: &Path, data: &HashMap<String, Vec<SessionEntry>>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(data) {
        let _ = std::fs::write(path, json);
    }
}

/// Dopisuje jedną sesję do historii danej gry — wywoływane RAZEM z
/// `add_minutes` (patrz `launcher::run_wrapped`), przy zamknięciu procesu
/// gry. Lista jest przycinana do `MAX_SESSIONS_PER_GAME` NAJNOWSZYCH
/// wpisów — bez limitu historia rosłaby bez końca przy regularnym graniu
/// przez lata. Podobnie jak `add_minutes`, zapisuje sesję nawet przy
/// `minutes == 0` (patrz jej dokumentacja po ten sam powód).
pub fn record_session(platform_slug: &str, game_id: &str, minutes: u64) {
    record_session_at(&sessions_store_path(), platform_slug, game_id, minutes);
}

fn record_session_at(path: &Path, platform_slug: &str, game_id: &str, minutes: u64) {
    let mut data = load_sessions_from(path);
    let list = data.entry(key(platform_slug, game_id)).or_default();
    list.push(SessionEntry { at: now_unix(), minutes });
    if list.len() > MAX_SESSIONS_PER_GAME {
        let excess = list.len() - MAX_SESSIONS_PER_GAME;
        list.drain(0..excess);
    }
    save_sessions_to(path, &data);
}

/// Historia sesji dla jednej gry, od najnowszej do najstarszej.
pub fn get_session_history(platform_slug: &str, game_id: &str) -> Vec<SessionEntry> {
    let mut sessions = load_sessions_from(&sessions_store_path())
        .remove(&key(platform_slug, game_id))
        .unwrap_or_default();
    sessions.sort_by(|a, b| b.at.cmp(&a.at));
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::stores::{Game, Platform};

    #[test]
    fn key_format_matches_platform_slug_and_game_id() {
        assert_eq!(key("epic", "abc123"), "epic:abc123");
    }

    fn sample_game(platform: Platform, id: &str, playtime_minutes: Option<u64>) -> Game {
        Game {
            id: id.to_string(),
            title: id.to_string(),
            platform,
            installed: true,
            install_dir: None,
            cover_path: None,
            playtime_minutes,
        }
    }

    #[test]
    fn merge_into_fills_only_missing_playtime() {
        let mut data = HashMap::new();
        data.insert(key("epic", "abc"), Entry { minutes: 90, last_played_at: 1000 });
        data.insert(key("steam", "123"), Entry { minutes: 999, last_played_at: 2000 });

        let mut games = vec![
            sample_game(Platform::Epic, "abc", None),
            sample_game(Platform::Steam, "123", Some(5)),
            sample_game(Platform::Gog, "999", None),
        ];

        merge_into_from(&data, &mut games);

        assert_eq!(games[0].playtime_minutes, Some(90));
        assert_eq!(games[1].playtime_minutes, Some(5));
        assert_eq!(games[2].playtime_minutes, None);
    }

    #[test]
    fn merge_into_noop_on_empty_data() {
        let data = HashMap::new();
        let mut games = vec![sample_game(Platform::Epic, "abc", None)];
        merge_into_from(&data, &mut games);
        assert_eq!(games[0].playtime_minutes, None);
    }

    #[test]
    fn add_minutes_persists_and_accumulates_across_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");

        add_minutes_at(&path, "epic", "abc", 30);
        add_minutes_at(&path, "epic", "abc", 15);
        add_minutes_at(&path, "gog", "999", 5);

        let data = load_from(&path);
        assert_eq!(data.get(&key("epic", "abc")).map(|e| e.minutes), Some(45));
        assert_eq!(data.get(&key("gog", "999")).map(|e| e.minutes), Some(5));
    }

    #[test]
    fn add_minutes_updates_last_played_even_when_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");

        add_minutes_at(&path, "epic", "abc", 0);

        let data = load_from(&path);
        let entry = data.get(&key("epic", "abc")).unwrap();
        assert_eq!(entry.minutes, 0);
        assert!(entry.last_played_at > 0);
    }

    #[test]
    fn load_from_missing_file_returns_empty_map() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn load_from_corrupted_file_falls_back_to_empty_map_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");
        std::fs::write(&path, "to nie jest poprawny JSON {{{").unwrap();
        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn load_from_reads_legacy_plain_number_format() {
        // Format sprzed dodania `last_played_at` — patrz komentarz przy
        // `Entry`/`RawEntry`. Test istnieje głównie po to, żeby dodanie
        // statystyk biblioteki nigdy przypadkiem nie skasowało czasu gry
        // zapisanego wcześniejszą wersją Hacker Mode.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");
        std::fs::write(&path, r#"{"epic:abc": 123}"#).unwrap();

        let data = load_from(&path);
        let entry = data.get("epic:abc").unwrap();
        assert_eq!(entry.minutes, 123);
        assert_eq!(entry.last_played_at, 0);
    }

    #[test]
    fn load_from_reads_mixed_legacy_and_new_format_in_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");
        std::fs::write(&path, r#"{"epic:old": 50, "gog:new": {"minutes": 80, "last_played_at": 1700000000}}"#)
            .unwrap();

        let data = load_from(&path);
        assert_eq!(data.get("epic:old").unwrap().minutes, 50);
        assert_eq!(data.get("gog:new").unwrap().minutes, 80);
        assert_eq!(data.get("gog:new").unwrap().last_played_at, 1700000000);
    }

    #[test]
    fn all_entries_splits_key_into_platform_and_game_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("playtime.json");
        add_minutes_at(&path, "epic", "abc123", 30);

        let data = load_from(&path);
        let entries: Vec<_> = data
            .into_iter()
            .filter_map(|(k, e)| {
                let (p, g) = k.split_once(':')?;
                Some((p.to_string(), g.to_string(), e.minutes))
            })
            .collect();
        assert_eq!(entries, vec![("epic".to_string(), "abc123".to_string(), 30)]);
    }

    #[test]
    fn record_session_appends_and_sorts_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");

        record_session_at(&path, "epic", "abc", 30);
        record_session_at(&path, "epic", "abc", 15);
        record_session_at(&path, "gog", "999", 5);

        let all = load_sessions_from(&path);
        let epic_sessions = all.get(&key("epic", "abc")).unwrap();
        assert_eq!(epic_sessions.len(), 2);
        assert_eq!(epic_sessions[0].minutes, 30);
        assert_eq!(epic_sessions[1].minutes, 15);
    }

    #[test]
    fn record_session_trims_to_max_sessions_per_game() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");

        for i in 0..(MAX_SESSIONS_PER_GAME + 10) {
            record_session_at(&path, "epic", "abc", i as u64);
        }

        let all = load_sessions_from(&path);
        let sessions = all.get(&key("epic", "abc")).unwrap();
        assert_eq!(sessions.len(), MAX_SESSIONS_PER_GAME);
        // Zostają NAJNOWSZE wpisy (najwyższe wartości `minutes`, bo
        // rosły wraz z pętlą) — najstarsze zostały odcięte.
        assert_eq!(sessions.last().unwrap().minutes, (MAX_SESSIONS_PER_GAME + 9) as u64);
    }

    #[test]
    fn get_session_history_returns_empty_for_unknown_game() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        record_session_at(&path, "epic", "abc", 10);
        // `get_session_history` (bez `_at`) czyta ze STAŁEJ ścieżki
        // (`sessions_store_path()`), więc tu testujemy tylko wczytanie z
        // pustego/nieistniejącego pliku przez `load_sessions_from`
        // bezpośrednio, ten sam kompromis co reszta testów w tym module.
        let empty = load_sessions_from(&dir.path().join("does-not-exist.json"));
        assert!(empty.get(&key("epic", "abc")).is_none());
    }
}
