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
}

impl AppState {
    pub fn new(dev_mode: bool) -> Self {
        Self {
            dev_mode: AtomicBool::new(dev_mode),
            settings: Mutex::new(Settings::load_or_default()),
            wrapper_active: AtomicBool::new(false),
        }
    }

    pub fn is_dev(&self) -> bool {
        self.dev_mode.load(Ordering::Relaxed)
    }
}
