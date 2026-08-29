use ribbit_client::{Endpoint, Region, RibbitClient};

/// Znane kody produktów Battle.net — potwierdzone z rzeczywistej,
/// aktualnej konfiguracji `Agent` (`locate_product_list`) zmirrorowanej
/// przez BlizzTrack (serwis społeczności data-miningowej, mirror realnych
/// configów z CDN Blizzarda): <https://blizztrack.com/config/agent/pc/>.
/// To NIE jest zgadywana/wymyślona lista — każdy kod niżej pochodzi z tego
/// źródła. Lista celowo nie jest wyczerpująca (Battle.net ma dziesiątki
/// wariantów regionalnych/testowych) — tylko najpopularniejsze produkty
/// końcowe, sensowne jako punkt startowy dla Hacker Mode.
pub const KNOWN_PRODUCT_CODES: &[(&str, &str)] = &[
    ("wow", "World of Warcraft"),
    ("wow_classic", "World of Warcraft Classic"),
    ("wow_classic_era", "World of Warcraft Classic Era"),
    ("d3", "Diablo III"),
    ("s1", "StarCraft"),
    ("s2", "StarCraft II"),
    ("w3", "Warcraft III: Reforged"),
    ("pro", "Overwatch"),
    ("hero", "Heroes of the Storm"),
    ("hsb", "Hearthstone"),
];

#[derive(Debug, Clone)]
pub struct ProductVersion {
    pub product_code: String,
    /// Region, dla którego zapytano (Ribbit zwraca dane per-region — różne
    /// regiony mogą mieć chwilowo różne wersje podczas wdrażania patcha).
    pub region: &'static str,
    /// Surowa odpowiedź BPSV (Blizzard Pipe-Separated Values) z Ribbit
    /// (`Endpoint::ProductVersions`), jako tekst — kolumny to m.in.
    /// `Region`, `BuildConfig`, `CDNConfig`, `BuildId`, `VersionsName`
    /// (patrz format BPSV, <https://wowdev.wiki/TACT#BPSV>). Świadomie
    /// zwracamy tu surowy tekst zamiast sparsowanych pól — parsowanie na
    /// konkretną strukturę i użycie `BuildConfig`/`CDNConfig` do pobrania
    /// plików z CDN (TACT) to zakres kolejnej sesji, patrz moduł-dokumentacja.
    pub raw_bpsv: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RibbitError {
    #[error("nie udało się połączyć z Ribbit ({host}:{port})")]
    ConnectionFailed { host: String, port: u16 },
    #[error("błąd protokołu Ribbit: {0}")]
    Protocol(String),
}

impl From<ribbit_client::Error> for RibbitError {
    fn from(err: ribbit_client::Error) -> Self {
        // `ribbit_client::Error` (patrz przykład w dokumentacji crate'a)
        // ma warianty typu `ConnectionFailed { host, port }`,
        // `ChecksumMismatch`, `MimeParseError(String)` — mapujemy je na
        // nasz uproszczony typ błędu zamiast przeciekać typ z zależności
        // przez publiczne API `bnet`, tak samo jak `EaError`/`BnetError`
        // w `lib.rs` robią to dla błędów Wine.
        RibbitError::Protocol(err.to_string())
    }
}

/// Odpytuje Ribbit o najnowszą dostępną wersję danego produktu
/// (`product_code` — patrz [`KNOWN_PRODUCT_CODES`]) w regionie US.
/// Nie wymaga żadnego logowania — Ribbit jest publicznym API discovery.
///
/// Używa `request_raw` (potwierdzone w dokumentacji `ribbit-client` jako
/// zwracające surowe bajty odpowiedzi, patrz cytat w module-doc tego
/// pliku) zamiast wyższopoziomowego `request`, którego dokładny kształt
/// zwracanego typu `Response` nie został w pełni zweryfikowany w tej
/// sesji (brak dostępu do pełnego źródła crate'a, tylko do fragmentów
/// dokumentacji) — bezpieczniej oprzeć się na udokumentowanym `Vec<u8>`
/// i sparsować BPSV ręcznie w kolejnej sesji, niż zgadywać nazwy pól
/// `Response`.
///
/// Asynchroniczne (jak cały `ribbit-client`) — wywołujący (patrz
/// `commands/stores/battlenet.rs` po stronie Hacker Mode) musi to odpalić
/// na runtime'ie tokio.
pub async fn query_product_versions(product_code: &str) -> Result<ProductVersion, RibbitError> {
    let client = RibbitClient::new(Region::US);
    let endpoint = Endpoint::ProductVersions(product_code.to_string());
    let raw = client.request_raw(&endpoint).await.map_err(RibbitError::from)?;

    Ok(ProductVersion {
        product_code: product_code.to_string(),
        region: "us",
        raw_bpsv: String::from_utf8_lossy(&raw).into_owned(),
    })
}
