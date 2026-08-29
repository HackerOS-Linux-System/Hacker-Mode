use std::process::Command;

/// Wysyła powiadomienie systemowe. `urgency` to jedno z `"low"`/
/// `"normal"`/`"critical"` (standardowe wartości `notify-send --urgency`)
/// — używane tak, by błędy (np. nieudana automatyczna kopia zapasowa)
/// były wizualnie odróżnialne od zwykłych "gotowe" bez konieczności
/// czytania treści.
pub fn send(summary: &str, body: &str, urgency: &str) {
    let result = Command::new("notify-send")
        .args(["--app-name=Hacker Mode", "--urgency", urgency, summary, body])
        .spawn();
    if let Err(err) = result {
        tracing::debug!(?err, "Nie udało się wysłać powiadomienia systemowego (notify-send niedostępne?)");
    }
}

pub fn info(summary: &str, body: &str) {
    send(summary, body, "normal");
}

pub fn error(summary: &str, body: &str) {
    send(summary, body, "critical");
}
