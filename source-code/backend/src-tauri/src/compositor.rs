use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

pub use crate::settings::Compositor;

static ACTIVE: OnceLock<Compositor> = OnceLock::new();

/// Kompozytor, na którym działamy (liczony raz).
pub fn active() -> Compositor {
    *ACTIVE.get_or_init(|| {
        if let Ok(value) = std::env::var("HACKER_MODE_COMPOSITOR") {
            if let Some(c) = Compositor::parse(&value) {
                return c;
            }
        }
        crate::settings::Settings::load_or_default().compositor
    })
}

/// Migawka identyfikatorów okien PRZED uruchomieniem procesu gry/launchera.
/// Dla `labwc` zawsze pusta (nie jest potrzebna).
pub fn snapshot_windows(compositor: Compositor, timeout: Duration) -> Vec<u64> {
    match compositor {
        Compositor::Comphwde => hacker_mode_ipc::known_window_ids(timeout).unwrap_or_default(),
        Compositor::Labwc => Vec::new(),
    }
}

/// Wejście w tryb wrapper po stronie kompozytora (poza ukryciem okna w
/// Tauri, które robi `launcher.rs`).
pub fn enter_wrapper(compositor: Compositor, timeout: Duration) {
    match compositor {
        Compositor::Comphwde => {
            if let Err(e) = hacker_mode_ipc::enter_wrapper(timeout) {
                tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
            }
        }
        // Ukryte (unmapped) okno powłoki znika z ekranu, a nowe okno gry
        // dostaje fokus i jest maksymalizowane regułą z rc.xml.
        Compositor::Labwc => {}
    }
}

/// Powrót z trybu wrapper (poza pokazaniem okna w Tauri).
pub fn exit_wrapper(compositor: Compositor, timeout: Duration) {
    match compositor {
        Compositor::Comphwde => {
            if let Err(e) = hacker_mode_ipc::exit_wrapper(timeout) {
                tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
            }
        }
        // Ponowne `show()` mapuje okno powłoki, a labwc nadaje fokus
        // nowo zmapowanym oknom.
        Compositor::Labwc => {}
    }
}

/// Proszy kompozytor o zakończenie sesji.
pub fn shutdown(compositor: Compositor, timeout: Duration) {
    match compositor {
        Compositor::Comphwde => {
            if let Err(e) = hacker_mode_ipc::shutdown_compositor(timeout) {
                tracing::debug!(error = %e, "Nie udało się poprosić comphwde o zamknięcie (być może już nie działa)");
            }
        }
        Compositor::Labwc => {
            // `labwc --exit` łączy się z działającą instancją przez
            // WAYLAND_DISPLAY/socket i prosi ją o wyjście.
            match std::process::Command::new("labwc").arg("--exit").status() {
                Ok(status) if status.success() => {}
                Ok(status) => tracing::debug!(?status, "`labwc --exit` zakończyło się niepowodzeniem"),
                Err(e) => tracing::debug!(error = %e, "Nie udało się uruchomić `labwc --exit`"),
            }
        }
    }
}

// ---------------------------------------------------------------------
// Ochrona przed wyścigiem "które okno jest nowe" (tylko comphwde)
// ---------------------------------------------------------------------

static MAXIMIZE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// RAII: dopóki istnieje, żadne inne uruchomienie nie próbuje
/// maksymalizować "nowego okna" (patrz [`try_begin_maximize`]).
pub struct MaximizeGuard(());

impl Drop for MaximizeGuard {
    fn drop(&mut self) {
        MAXIMIZE_IN_FLIGHT.store(false, Ordering::SeqCst);
    }
}

/// `maximize_next_new_window` dopasowuje "pierwsze okno, którego nie było w
/// migawce". Gdy dwa procesy startują niemal jednocześnie, migawka drugiego
/// mogłaby zostać zrobiona PRZED zmapowaniem okna pierwszego, a wtedy drugi
/// wątek zmaksymalizowałby okno pierwszej gry. Zamiast zgadywać, tylko
/// jedno uruchomienie naraz dostaje asystę maksymalizacji; pozostałe ją
/// pomijają (bezpieczniej: okno zostaje niezmaksymalizowane, ale żadne
/// INNE okno nie jest ruszane).
pub fn try_begin_maximize() -> Option<MaximizeGuard> {
    MAXIMIZE_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| MaximizeGuard(()))
}

/// Czeka na nowe okno i maksymalizuje je (comphwde). Dla `labwc` robi to
/// reguła w `rc.xml`, więc zwraca `Ok(None)`.
pub fn maximize_new_window(
    compositor: Compositor,
    known: &[u64],
    timeout: Duration,
    poll: Duration,
) -> Result<Option<u64>, String> {
    match compositor {
        Compositor::Comphwde => hacker_mode_ipc::maximize_next_new_window(known, timeout, poll)
            .map(Some)
            .map_err(|e| e.to_string()),
        Compositor::Labwc => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `MAXIMIZE_IN_FLIGHT` jest globalny - wszystkie asercje o nim w jednym teście.
    #[test]
    fn only_one_maximize_assist_at_a_time() {
        let first = try_begin_maximize().expect("pierwsze uruchomienie dostaje asystę");
        assert!(try_begin_maximize().is_none(), "drugie równoległe musi ją pominąć");
        drop(first);
        assert!(try_begin_maximize().is_some(), "po zwolnieniu można znów");
    }

    #[test]
    fn labwc_operations_are_noops() {
        assert!(snapshot_windows(Compositor::Labwc, Duration::from_millis(10)).is_empty());
        assert_eq!(
            maximize_new_window(Compositor::Labwc, &[], Duration::from_millis(10), Duration::from_millis(5)),
            Ok(None)
        );
        enter_wrapper(Compositor::Labwc, Duration::from_millis(10));
        exit_wrapper(Compositor::Labwc, Duration::from_millis(10));
    }
}
