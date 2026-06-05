use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::db::{games::GameRow, Database};
use crate::runner::RunnerType;
use crate::store::StoreSource;

// ── Install type ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SteamInstallType {
    Native,
    Snap,
    Flatpak,
}

#[derive(Debug, Clone)]
pub struct SteamInstall {
    pub kind: SteamInstallType,
    /// Root of the Steam data directory, e.g. ~/.steam/steam
    pub root: PathBuf,
    /// Binary to launch Steam/games
    pub binary: PathBuf,
}

impl SteamInstall {
    /// Detect which variant of Steam is installed.
    pub fn detect() -> Option<Self> {
        // 1. Native
        let native_root = dirs::home_dir()?.join(".steam/steam");
        if native_root.is_dir() {
            let bin = PathBuf::from("/usr/bin/steam");
            let bin = if bin.exists() { bin } else { PathBuf::from("steam") };
            info!("Detected native Steam at {}", native_root.display());
            return Some(Self { kind: SteamInstallType::Native, root: native_root, binary: bin });
        }

        // 2. Flatpak
        let flatpak_root = dirs::home_dir()?
            .join(".var/app/com.valvesoftware.Steam/data/Steam");
        if flatpak_root.is_dir() {
            info!("Detected Flatpak Steam at {}", flatpak_root.display());
            return Some(Self {
                kind: SteamInstallType::Flatpak,
                root: flatpak_root,
                binary: PathBuf::from("flatpak"),
            });
        }

        // 3. Snap
        let snap_root = PathBuf::from("/snap/steam/current/usr/lib/steam");
        if snap_root.is_dir() {
            info!("Detected Snap Steam at {}", snap_root.display());
            return Some(Self {
                kind: SteamInstallType::Snap,
                root: snap_root,
                binary: PathBuf::from("snap"),
            });
        }

        warn!("No Steam installation detected");
        None
    }

    /// All libraryfolders paths (can be multiple drives).
    pub fn library_paths(&self) -> Vec<PathBuf> {
        let vdf = self.root.join("steamapps/libraryfolders.vdf");
        parse_library_folders(&vdf).unwrap_or_else(|e| {
            warn!("Failed to parse libraryfolders.vdf: {e}");
            vec![self.root.join("steamapps")]
        })
    }

    /// Cover art local cache path for an appid.
    pub fn cover_path(&self, appid: u32) -> PathBuf {
        self.root
            .join(format!("appcache/librarycache/{appid}_library_600x900.jpg"))
    }

    pub fn hero_path(&self, appid: u32) -> PathBuf {
        self.root
            .join(format!("appcache/librarycache/{appid}_library_hero.jpg"))
    }
}

// ── VDF parser ────────────────────────────────────────────────────────────────

/// Very small hand-rolled KeyValues / VDF parser – handles the subset of
/// Valve's text format used by libraryfolders.vdf and appmanifest_*.acf.
/// We do NOT pull in a heavy dependency for 200 lines of KV parsing.
fn parse_kv_flat(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    // Matches:  "key"   "value"
    let re = regex::Regex::new(r#""([^"]+)"\s+"([^"]*)""#).unwrap();
    for cap in re.captures_iter(content) {
        map.insert(cap[1].to_lowercase(), cap[2].to_owned());
    }
    map
}

fn parse_library_folders(vdf_path: &Path) -> Result<Vec<PathBuf>> {
    let content = std::fs::read_to_string(vdf_path)
        .with_context(|| format!("Reading {}", vdf_path.display()))?;

    let mut paths = Vec::new();
    // Modern format has nested "path" keys inside numbered sections
    let path_re = regex::Regex::new(r#""path"\s+"([^"]+)""#).unwrap();
    for cap in path_re.captures_iter(&content) {
        let p = PathBuf::from(&cap[1]).join("steamapps");
        if p.is_dir() {
            paths.push(p);
        }
    }
    // Older flat format: "1" "/path/to/library"
    let legacy_re = regex::Regex::new(r#""\d+"\s+"([^"]+)""#).unwrap();
    for cap in legacy_re.captures_iter(&content) {
        let p = PathBuf::from(&cap[1]).join("steamapps");
        if p.is_dir() && !paths.contains(&p) {
            paths.push(p);
        }
    }

    if paths.is_empty() {
        bail!("No library paths found in {}", vdf_path.display());
    }
    Ok(paths)
}

// ── App manifest ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AppManifest {
    pub appid: u32,
    pub name: String,
    pub install_dir: String,
    pub size_on_disk: u64,
    pub last_played: Option<u64>,
}

fn parse_manifest(path: &Path) -> Result<AppManifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Reading manifest {}", path.display()))?;
    let kv = parse_kv_flat(&content);

    let appid: u32 = kv.get("appid")
        .and_then(|v| v.parse().ok())
        .context("Missing appid in manifest")?;
    let name = kv.get("name").cloned().unwrap_or_else(|| format!("Unknown ({})", appid));
    let install_dir = kv.get("installdir").cloned().unwrap_or_default();
    let size_on_disk: u64 = kv.get("sizeonddisk").and_then(|v| v.parse().ok()).unwrap_or(0);
    let last_played: Option<u64> = kv.get("lastplayed").and_then(|v| v.parse().ok());

    Ok(AppManifest { appid, name, install_dir, size_on_disk, last_played })
}

// ── Import ────────────────────────────────────────────────────────────────────

pub struct SteamImporter {
    pub install: SteamInstall,
}

impl SteamImporter {
    pub fn new() -> Option<Self> {
        SteamInstall::detect().map(|install| Self { install })
    }

    /// Scan all library paths and upsert every installed game into the DB.
    pub async fn import_all(&self, db: &Database) -> Result<usize> {
        let mut count = 0usize;
        for lib_path in self.install.library_paths() {
            debug!("Scanning steamapps at {}", lib_path.display());
            let mut read_dir = tokio::fs::read_dir(&lib_path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                    continue;
                }
                match parse_manifest(&entry.path()) {
                    Ok(manifest) => {
                        let install_path = lib_path.join("common").join(&manifest.install_dir);
                        let row = build_game_row(&manifest, &install_path, &self.install);
                        crate::db::games::upsert(db, &row).await?;
                        count += 1;
                        debug!("Imported: {} ({})", manifest.name, manifest.appid);
                    }
                    Err(e) => warn!("Skipping {}: {e}", entry.path().display()),
                }
            }
        }
        info!("Steam import complete: {count} games");
        Ok(count)
    }
}

fn build_game_row(manifest: &AppManifest, install_path: &Path, install: &SteamInstall) -> GameRow {
    let mut row = GameRow::new(
        manifest.appid.to_string(),
        StoreSource::Steam,
        &manifest.name,
        &RunnerType::SteamNative { appid: manifest.appid },
    );
    row.install_path = Some(install_path.to_string_lossy().into_owned());
    row.size_bytes = Some(manifest.size_on_disk as i64);
    row.last_played = manifest.last_played.map(|t| t as i64);
    row.is_installed = install_path.is_dir();

    let cover = install.cover_path(manifest.appid);
    if cover.exists() {
        row.cover_path = Some(cover.to_string_lossy().into_owned());
    }
    let hero = install.hero_path(manifest.appid);
    if hero.exists() {
        row.hero_path = Some(hero.to_string_lossy().into_owned());
    }
    row
}

// ── Launcher ─────────────────────────────────────────────────────────────────

pub async fn launch(install: &SteamInstall, appid: u32) -> Result<tokio::process::Child> {
    info!("Launching Steam appid {appid} via {:?}", install.kind);
    let child = match install.kind {
        SteamInstallType::Native => {
            Command::new(&install.binary)
                .arg("-applaunch")
                .arg(appid.to_string())
                .spawn()?
        }
        SteamInstallType::Flatpak => {
            Command::new("flatpak")
                .args(["run", "com.valvesoftware.Steam", "-applaunch", &appid.to_string()])
                .spawn()?
        }
        SteamInstallType::Snap => {
            Command::new("snap")
                .args(["run", "steam", "--", "-applaunch", &appid.to_string()])
                .spawn()?
        }
    };
    Ok(child)
}
