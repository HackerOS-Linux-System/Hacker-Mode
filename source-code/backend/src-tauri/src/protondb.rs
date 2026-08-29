use serde::{Deserialize, Serialize};

/// Skrócone podsumowanie oceny — pełna odpowiedź ProtonDB ma więcej pól
/// (rozkład zgłoszeń po wersji Protona, trend w czasie), ale widok
/// szczegółów gry (`GameDetail.tsx`) potrzebuje tylko plakietki z ogólną
/// oceną, więc reszty tu nie odtwarzamy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonDbSummary {
    /// Jedna z: "platinum", "gold", "silver", "bronze", "borked",
    /// "pending" — surowa wartość z API, przekazywana do frontendu bez
    /// tłumaczenia (patrz `PROTONDB_TIER_LABELS`/kolory w
    /// `GameDetail.tsx`, gdzie te same stringi są mapowane na etykiety i
    /// kolor plakietki).
    pub tier: String,
    pub confidence: String,
    pub score: f64,
    pub total: u64,
}

fn parse_summary(body: &str) -> Option<ProtonDbSummary> {
    serde_json::from_str(body).ok()
}

/// Pobiera ocenę ProtonDB dla danego Steam AppID. Zwraca `None` zarówno
/// gdy gra nie ma jeszcze żadnych zgłoszeń w ProtonDB (częste dla nowych/
/// niszowych gier — API wtedy odpowiada 404), jak i przy jakimkolwiek
/// błędzie sieciowym — z punktu widzenia UI (`GameDetail.tsx`) to i tak to
/// samo: po prostu nie pokazujemy plakietki zamiast pokazywać błąd.
pub fn fetch_rating(appid: u64) -> Option<ProtonDbSummary> {
    let client = crate::cover_cache::build_client()?;
    let url = format!("https://www.protondb.com/api/v1/reports/summaries/{appid}.json");
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        // 404 = brak zgłoszeń dla tej gry w ProtonDB, nie błąd — logujemy
        // tylko na poziomie debug, nie warn, żeby nie zaśmiecać logów przy
        // każdej niszowej grze w bibliotece.
        tracing::debug!(appid, status = %response.status(), "ProtonDB: brak oceny dla tej gry");
        return None;
    }
    let body = response.text().ok()?;
    parse_summary(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kształt oparty na publicznie widocznych odpowiedziach API ProtonDB
    /// (używanych np. przez rozszerzenia przeglądarki pokazujące plakietkę
    /// na stronie sklepu Steam) — niezweryfikowane na żywym wywołaniu w
    /// tym środowisku (patrz README, „Do zrobienia dalej”).
    const SAMPLE_GOLD: &str = r#"{
        "bestReportedTier": "platinum",
        "confidence": "strong",
        "score": 0.78,
        "tier": "gold",
        "total": 1523,
        "trendingTier": "gold"
    }"#;

    #[test]
    fn parses_summary_response() {
        let summary = parse_summary(SAMPLE_GOLD).unwrap();
        assert_eq!(summary.tier, "gold");
        assert_eq!(summary.confidence, "strong");
        assert_eq!(summary.total, 1523);
        assert!((summary.score - 0.78).abs() < f64::EPSILON);
    }

    #[test]
    fn garbage_input_returns_none_instead_of_panicking() {
        assert!(parse_summary("nie json").is_none());
        assert!(parse_summary("").is_none());
    }
}
