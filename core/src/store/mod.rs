pub mod steam;
pub mod epic;
pub mod gog;
pub mod amazon;
pub mod itchio;
pub mod lutris;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoreSource {
    Steam,
    Epic,
    Gog,
    Amazon,
    Itchio,
    Lutris,
    Native,
}

impl StoreSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreSource::Steam   => "steam",
            StoreSource::Epic    => "epic",
            StoreSource::Gog     => "gog",
            StoreSource::Amazon  => "amazon",
            StoreSource::Itchio  => "itchio",
            StoreSource::Lutris  => "lutris",
            StoreSource::Native  => "native",
        }
    }

    pub fn store_url(&self) -> Option<&'static str> {
        match self {
            StoreSource::Steam   => Some("https://store.steampowered.com"),
            StoreSource::Epic    => Some("https://store.epicgames.com"),
            StoreSource::Gog     => Some("https://www.gog.com"),
            StoreSource::Amazon  => Some("https://gaming.amazon.com"),
            StoreSource::Itchio  => Some("https://itch.io"),
            StoreSource::Lutris  => Some("https://lutris.net"),
            StoreSource::Native  => None,
        }
    }
}

impl std::fmt::Display for StoreSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for StoreSource {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "steam"   => Ok(StoreSource::Steam),
            "epic"    => Ok(StoreSource::Epic),
            "gog"     => Ok(StoreSource::Gog),
            "amazon"  => Ok(StoreSource::Amazon),
            "itchio"  => Ok(StoreSource::Itchio),
            "lutris"  => Ok(StoreSource::Lutris),
            "native"  => Ok(StoreSource::Native),
            other     => Err(anyhow::anyhow!("unknown source: {other}")),
        }
    }
}
