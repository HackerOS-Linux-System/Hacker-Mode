use tauri::{AppHandle, Emitter, WebviewUrl, WebviewWindowBuilder};

use crate::commands::stores::{self, Platform};

/// Otwiera okno logowania dla danej platformy. Dla Steam (które nie ma
/// żadnego kodu do przechwycenia — logowanie odbywa się w natywnym kliencie
/// Steam) po prostu odpala klienta zamiast okna webview.
pub fn open_login_window(app: AppHandle, platform: Platform) -> Result<(), String> {
    let flow = stores::login_start(platform).map_err(|e| e.to_string())?;

    let Some(url) = flow.url else {
        // Brak URL-a logowania (np. Steam) — komunikat już wyjaśnia co
        // zrobić (otwórz klienta Steam), więc tylko go odpalamy.
        let _ = std::process::Command::new("steam").spawn();
        return Ok(());
    };

    let window_label = format!("login-{}", platform_slug(platform));
    let app_for_nav = app.clone();

    // Amazon (`nile`) nie kończy logowania przez przekierowanie URL-a z
    // kodem (jak GOG) ani przez stronę z JSON-em do wklejenia (jak Epic) —
    // sam proces `nile auth --login`, uruchomiony w `AmazonProvider::login_start`,
    // blokuje do czasu zalogowania i kończy się sukcesem/błędem. Doczekujemy
    // go tutaj w tle i emitujemy wynik, zamiast zostawiać go bez nadzoru.
    if platform == Platform::Amazon {
        let app_for_wait = app.clone();
        std::thread::spawn(move || {
            let Some(mut child) = stores::amazon::take_pending_login_child() else {
                tracing::warn!("Brak oczekującego procesu logowania `nile` do doczekania");
                return;
            };
            let status = child.wait();
            let ok = status.map(|s| s.success()).unwrap_or(false);
            if !ok {
                tracing::warn!("Logowanie do Amazon Games (`nile auth --login`) nie powiodło się");
            }
            if let Some(win) = app_for_wait.get_webview_window_compat(&window_label_for_close(Platform::Amazon)) {
                let _ = win.close();
            }
            let _ = app_for_wait.emit(
                "hacker-mode://store-login-finished",
                serde_json::json!({ "platform": Platform::Amazon, "ok": ok }),
            );
        });
    }

    let parsed_url = url
        .parse()
        .map_err(|e| format!("Nieprawidłowy URL logowania: {e}"))?;

    WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(parsed_url))
        .title(format!("Logowanie — {}", platform.label()))
        .inner_size(900.0, 720.0)
        .on_navigation(move |navigated_url| {
            if let Some(code) = extract_code_param(navigated_url.as_str()) {
                tracing::info!(platform = ?platform, "Wykryto kod autoryzacyjny w URL-u przekierowania");
                complete_login(app_for_nav.clone(), platform, code, window_label_for_close(platform));
            }
            // Zawsze pozwalamy nawigacji kontynuować — to tylko podsłuch,
            // nie blokada.
            true
        })
        .build()
        .map_err(|e| format!("Nie udało się otworzyć okna logowania: {e}"))?;

    Ok(())
}

/// Wywoływane z frontendu, gdy użytkownik ręcznie wkleja kod (flow Epic —
/// strona po zalogowaniu pokazuje JSON zamiast przekierowywać z parametrem
/// w adresie, więc auto-detekcja z `on_navigation` nie zadziała).
pub fn submit_login_code(app: AppHandle, platform: Platform, code: String) -> Result<(), String> {
    stores::login_submit(platform, &code).map_err(|e| e.to_string())?;
    close_login_window(&app, platform);
    let _ = app.emit(
        "hacker-mode://store-login-finished",
        serde_json::json!({ "platform": platform, "ok": true }),
    );
    Ok(())
}

fn complete_login(app: AppHandle, platform: Platform, code: String, window_label: String) {
    std::thread::spawn(move || {
        let result = stores::login_submit(platform, &code);
        let ok = result.is_ok();
        if let Err(err) = result {
            tracing::warn!(?platform, %err, "Logowanie nie powiodło się");
        }
        if let Some(win) = app.get_webview_window_compat(&window_label) {
            let _ = win.close();
        }
        let _ = app.emit(
            "hacker-mode://store-login-finished",
            serde_json::json!({ "platform": platform, "ok": ok }),
        );
    });
}

fn close_login_window(app: &AppHandle, platform: Platform) {
    if let Some(win) = app.get_webview_window_compat(&window_label_for_close(platform)) {
        let _ = win.close();
    }
}

fn window_label_for_close(platform: Platform) -> String {
    format!("login-{}", platform_slug(platform))
}

fn platform_slug(platform: Platform) -> &'static str {
    match platform {
        Platform::Steam => "steam",
        Platform::Epic => "epic",
        Platform::Gog => "gog",
        Platform::Amazon => "amazon",
        Platform::Lutris => "lutris",
    }
}

/// Wyciąga wartość parametru `code` z query stringa URL-a, bez dodawania
/// zależności od crate `url` — format jest prosty (`?a=b&code=XYZ&c=d`).
fn extract_code_param(url: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next()?;
        if key == "code" {
            return Some(urldecode(value));
        }
    }
    None
}

/// Minimalne url-decode (`%XX` -> bajt, `+` -> spacja) — wystarczające dla
/// kodów autoryzacyjnych (zwykle same znaki alfanumeryczne, ale bywają
/// URL-encodowane znaki specjalne w zależności od implementacji OAuth).
fn urldecode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
                result.push('%');
                i += 1;
            }
            b'+' => {
                result.push(' ');
                i += 1;
            }
            b => {
                result.push(b as char);
                i += 1;
            }
        }
    }
    result
}

/// Mała warstwa kompatybilności: w Tauri v2 pobranie okna po etykiecie to
/// `app.get_webview_window(label)`. Nazwa pomocnicza tutaj tylko po to, by
/// nie powtarzać importu `tauri::Manager` w kilku miejscach tego pliku.
trait WindowLookupCompat {
    fn get_webview_window_compat(&self, label: &str) -> Option<tauri::WebviewWindow>;
}

impl WindowLookupCompat for AppHandle {
    fn get_webview_window_compat(&self, label: &str) -> Option<tauri::WebviewWindow> {
        use tauri::Manager;
        self.get_webview_window(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_code_from_query_string() {
        let url = "https://embed.gog.com/on_login_success?origin=client&code=abc123";
        assert_eq!(extract_code_param(url), Some("abc123".to_string()));
    }

    #[test]
    fn returns_none_without_code_param() {
        let url = "https://www.epicgames.com/id/api/redirect?clientId=xyz&responseType=code";
        assert_eq!(extract_code_param(url), None);
    }

    #[test]
    fn urldecodes_special_characters() {
        assert_eq!(urldecode("a%20b%2Bc"), "a b+c");
    }
}
