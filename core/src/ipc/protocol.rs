use serde::{Deserialize, Serialize};
use crate::db::games::GameRow;
use crate::perf::PerfSnapshot;
use crate::settings::network::{NetworkConnection, WifiNetwork};

// ── Requests ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    // Library
    GetAllGames,
    GetGamesBySource { source: String },
    GetGame          { id: String },
    SetFavourite     { id: String, favourite: bool },

    // Launch
    LaunchGame  { id: String },
    KillGame    { id: String },
    RunningGames,

    // Import
    ImportSteam,
    ImportEpic,
    ImportGog,
    ImportAmazon,
    ImportItchio,
    ImportLutris,
    ImportAll,

    // Proton
    ListProtonVersions,
    InstallGeProton,

    // Settings
    GetConfig,
    SetConfig { config: serde_json::Value },

    // Network
    ListConnections,
    ListWifi,
    ConnectWifi { ssid: String, password: Option<String> },
    Disconnect  { device: String },

    // Power: "shutdown"|"reboot"|"suspend"|"hibernate"|"logout"|"restart_hm"|"switch_to:<DE>"
    PowerAction { action: String },

    // Perf
    GetPerfSnapshot,
}

// ── Responses ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
    Games(Vec<GameRow>),
    Game(Option<GameRow>),
    Ok,
    Count(usize),
    ProtonVersions(Vec<String>),
    PerfSnapshot(PerfSnapshot),
    NetworkConnections(Vec<NetworkConnection>),
    WifiNetworks(Vec<WifiNetwork>),
    Config(serde_json::Value),
    RunningGameIds(Vec<String>),
    Error(String),
}

// ── Server-push events ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    PerfUpdate(PerfSnapshot),
    GameLaunched    { id: String, pid: u32 },
    GameExited      { id: String },
    ImportProgress  { source: String, count: usize },
    Notification    { title: String, body: String },
}
