use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub name:   String,
    pub kind:   String,   // "wifi", "ethernet", "vpn", …
    pub state:  String,   // "activated", "activating", "deactivated"
    pub device: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid:     String,
    pub signal:   u32,    // 0-100
    pub security: String, // "WPA2", "WPA3", "", …
    pub in_use:   bool,
}

async fn nmcli(args: &[&str]) -> Result<String> {
    let out = Command::new("nmcli")
        .args(args)
        .output()
        .await
        .context("nmcli not found – is NetworkManager installed?")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        anyhow::bail!("nmcli error: {err}");
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub async fn list_connections() -> Result<Vec<NetworkConnection>> {
    let raw = nmcli(&["-t", "-f", "NAME,TYPE,STATE,DEVICE", "connection", "show"]).await?;
    let conns = raw.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() < 4 { return None; }
            Some(NetworkConnection {
                name:   parts[0].to_owned(),
                kind:   parts[1].to_owned(),
                state:  parts[2].to_owned(),
                device: parts[3].to_owned(),
            })
        })
        .collect();
    Ok(conns)
}

pub async fn list_wifi() -> Result<Vec<WifiNetwork>> {
    let raw = nmcli(&["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list"]).await?;
    let networks = raw.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() < 4 { return None; }
            let ssid = parts[1].to_owned();
            if ssid.is_empty() { return None; } // hidden SSID
            Some(WifiNetwork {
                in_use:   parts[0] == "*",
                ssid,
                signal:   parts[2].parse().unwrap_or(0),
                security: parts[3].to_owned(),
            })
        })
        .collect();
    Ok(networks)
}

pub async fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<()> {
    let mut args = vec!["device", "wifi", "connect", ssid];
    let owned_pw; // keep alive
    if let Some(pw) = password {
        owned_pw = pw.to_owned();
        args.extend(["password", owned_pw.as_str()]);
    }
    nmcli(&args).await?;
    Ok(())
}

pub async fn disconnect(device: &str) -> Result<()> {
    nmcli(&["device", "disconnect", device]).await?;
    Ok(())
}

pub async fn active_connection() -> Option<NetworkConnection> {
    list_connections().await.ok()?
        .into_iter()
        .find(|c| c.state == "activated")
}
