use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("hacker-mode")
        .join("covers")
}

/// Pobiera obraz spod `url` i zapisuje go w lokalnym cache pod `cache_key`
/// (zwykle `<platforma>-<id_gry>`), o ile jeszcze go tam nie ma. Zwraca
/// ścieżkę do lokalnego pliku. Błędy sieciowe są ciche (zwracają `None`) —
/// brak okładki nie powinien nigdy blokować wyświetlenia gry w bibliotece.
pub fn cache_image(url: &str, cache_key: &str) -> Option<String> {
    let dir = cache_dir();
    let ext = url.rsplit('.').next().filter(|e| e.len() <= 4).unwrap_or("jpg");
    let path = dir.join(format!("{cache_key}.{ext}"));

    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }

    std::fs::create_dir_all(&dir).ok()?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("HackerMode/0.3")
        .build()
        .ok()?;

    let response = client.get(url).send().ok()?;
    if !response.status().is_success() {
        tracing::debug!(url, status = %response.status(), "Nie udało się pobrać okładki");
        return None;
    }
    let bytes = response.bytes().ok()?;

    let mut file = std::fs::File::create(&path).ok()?;
    file.write_all(&bytes).ok()?;

    Some(path.to_string_lossy().into_owned())
}

/// Heurystycznie przeszukuje dowolny dokument JSON w poszukiwaniu
/// pierwszej wartości tekstowej wyglądającej na URL obrazka. Preferuje
/// klucze zawierające "image"/"logo"/"box"/"cover"/"thumb" w nazwie (typowe
/// nazewnictwo w API sklepów gier), ale w razie ich braku bierze
/// jakikolwiek pasujący URL.
pub fn find_image_url(value: &serde_json::Value) -> Option<String> {
    fn is_image_url(s: &str) -> bool {
        (s.starts_with("http") || s.starts_with("//"))
            && ["jpg", "jpeg", "png", "webp"]
                .iter()
                .any(|ext| s.to_lowercase().ends_with(ext))
    }

    fn walk(value: &serde_json::Value, preferred: &mut Option<String>, fallback: &mut Option<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, v) in map {
                    if let serde_json::Value::String(s) = v {
                        if is_image_url(s) {
                            let key_lower = key.to_lowercase();
                            if preferred.is_none()
                                && ["image", "logo", "box", "cover", "thumb", "background"]
                                    .iter()
                                    .any(|hint| key_lower.contains(hint))
                            {
                                *preferred = Some(s.clone());
                            }
                            if fallback.is_none() {
                                *fallback = Some(s.clone());
                            }
                        }
                    } else {
                        walk(v, preferred, fallback);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    walk(item, preferred, fallback);
                }
            }
            _ => {}
        }
    }

    let mut preferred = None;
    let mut fallback = None;
    walk(value, &mut preferred, &mut fallback);
    preferred.or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_key_hinting_at_cover_art() {
        let doc = json!({
            "banner": "https://example.com/banner.jpg",
            "images": { "logo2x": "https://example.com/logo.png" }
        });
        assert_eq!(find_image_url(&doc), Some("https://example.com/logo.png".to_string()));
    }

    #[test]
    fn falls_back_to_any_image_url() {
        let doc = json!({ "misc": "https://example.com/pic.webp" });
        assert_eq!(find_image_url(&doc), Some("https://example.com/pic.webp".to_string()));
    }

    #[test]
    fn returns_none_without_any_image_url() {
        let doc = json!({ "title": "Some Game", "id": 42 });
        assert_eq!(find_image_url(&doc), None);
    }
}
