use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::settings::Settings;

pub struct AppState {
    /// `true` gdy uruchomiono jako `hacker-mode dev` — wyłącza integrację
    /// z kompozytorem (fullscreen kiosk, IPC) i większość akcji
    /// systemowych (power/shutdown), tak by dało się bezpiecznie testować
    /// UI na zwykłym pulpicie.
    pub dev_mode: AtomicBool,
    pub settings: Mutex<Settings>,
    /// Czy tryb "wrapper" (zamykanie Hacker Mode na czas działania
    /// zewnętrznej aplikacji, tak jak w wersji 0.1) jest aktualnie aktywny.
    pub wrapper_active: AtomicBool,
    /// PID procesu aktualnie uruchomionej gry, per gra — klucz
    /// `"<platform-slug>:<game-id>"` (ten sam format co
    /// `cover_override_key`). Wypełniane w `launcher::launch_game` zaraz
    /// po `spawn()`, usuwane w `run_wrapped`, gdy proces się zakończy.
    ///
    /// Używane przez `commands::stop_game` do znalezienia PID-u do
    /// zabicia — i przez `commands::is_game_running` (odpytywane przez
    /// frontend zaraz po starcie aplikacji, żeby od razu wiedzieć, czy
    /// jakaś gra już działa, np. po odświeżeniu strony/przełączeniu
    /// widoku, bez czekania na kolejne zdarzenie `game-launched`, które
    /// wtedy by już nie nadeszło — zdarzenia Tauri nie mają historii,
    /// nowy listener nie dostaje zdarzeń wyemitowanych PRZED jego
    /// zarejestrowaniem).
    ///
    /// UWAGA (patrz `launcher.rs`/`commands::stop_game`): dla Steam ten
    /// PID to proces polecenia `steam -applaunch <appid>`, NIE proces
    /// samej gry — jeśli Steam już działał, ten proces kończy się niemal
    /// natychmiast po przekazaniu żądania działającemu klientowi (Steam
    /// używa IPC do komunikacji między instancjami), więc dla Steam ten
    /// wpis w praktyce znika z mapy sekundy po starcie, mimo że gra
    /// dalej działa. To ograniczenie samego mechanizmu Steam, nie błąd —
    /// udokumentowane też w UI (`GameDetail.tsx`/`GameCard.tsx` nie
    /// pokazują przycisku "Zatrzymaj" dla Steam z tego właśnie powodu).
    pub running_games: Mutex<HashMap<String, u32>>,
    /// Gry Steam, których działanie zostało POTWIERDZONE przez oficjalne
    /// Steam Web API (`ISteamUser/GetPlayerSummaries`), niezależnie od
    /// `running_games` powyżej — patrz moduł-dokumentacja
    /// `launcher::watch_steam_web_api_session`. Klucz w tym samym
    /// formacie `"<platform-slug>:<game-id>"` co `running_games` (dla
    /// Steam zawsze `"steam:<appid>"`), tak żeby `is_game_running` mogło
    /// sprawdzić OBIE mapy tym samym kluczem.
    ///
    /// Istnieje jako OSOBNA mapa, nie jako kolejny wpis w
    /// `running_games` (który trzyma PID-y, `u32`) — bo śledzenie przez
    /// Web API nie ma żadnego PID-u do zapisania (patrz ograniczenie
    /// opisane przy `running_games`: proces `steam -applaunch` kończy
    /// się na długo przed zamknięciem samej gry, więc jego PID i tak nie
    /// nadaje się do niczego poza tym oknem czasowym). Gdy ten zestaw
    /// zawiera dany klucz, `commands::stop_game` wciąż nie ma jak
    /// faktycznie zatrzymać gry (brak PID-u) — status "w trakcie gry"
    /// jest teraz dokładniejszy, ale przycisk "Zatrzymaj" dla Steam
    /// pozostaje wyłączony z tego samego powodu co wcześniej.
    pub steam_watched_games: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new(dev_mode: bool) -> Self {
        Self {
            dev_mode: AtomicBool::new(dev_mode),
            settings: Mutex::new(Settings::load_or_default()),
            wrapper_active: AtomicBool::new(false),
            running_games: Mutex::new(HashMap::new()),
            steam_watched_games: Mutex::new(HashSet::new()),
        }
    }

    pub fn is_dev(&self) -> bool {
        self.dev_mode.load(Ordering::Relaxed)
    }
}
