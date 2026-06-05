use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, debug};

// ── Proton version discovery ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProtonVersion {
    pub name: String,
    pub path: PathBuf,
}

/// Find all Proton versions installed under Steam compatibility tools.
pub fn find_proton_versions() -> Vec<ProtonVersion> {
    let mut versions = Vec::new();
    let search_paths = [
        // Official Proton (under steamapps)
        dirs::home_dir()
            .map(|h| h.join(".steam/steam/steamapps/common")),
        // GE-Proton / custom tools
        dirs::home_dir()
            .map(|h| h.join(".steam/root/compatibilitytools.d")),
        // XDG data dir installs
        dirs::data_dir()
            .map(|d| d.join("Steam/compatibilitytools.d")),
    ];

    for path_opt in search_paths.iter().flatten() {
        let Ok(entries) = std::fs::read_dir(path_opt) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            // A valid Proton version has a 'proton' executable
            let exe = path.join("proton");
            if exe.exists() {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                debug!("Found Proton: {name} at {}", path.display());
                versions.push(ProtonVersion { name, path });
            }
        }
    }
    // Sort by name descending (newest first)
    versions.sort_by(|a, b| b.name.cmp(&a.name));
    versions
}

pub fn find_proton_by_name(name: &str) -> Option<ProtonVersion> {
    find_proton_versions().into_iter().find(|v| v.name == name)
}

// ── Proton launcher ───────────────────────────────────────────────────────────

/// Launch a Windows executable via a specific Proton version.
pub async fn launch(
    exe: &str,
    proton_version: &str,
    prefix: &Path,
    working_dir: Option<&str>,
) -> Result<tokio::process::Child> {
    let version = find_proton_by_name(proton_version)
        .with_context(|| format!("Proton version '{proton_version}' not found"))?;

    let proton_bin = version.path.join("proton");
    info!("Launching '{}' with {} (prefix: {})", exe, proton_version, prefix.display());

    // Ensure prefix dir exists
    tokio::fs::create_dir_all(prefix).await?;

    let _steam_compat_data = prefix.parent().unwrap_or(prefix);
    let steam_install = dirs::home_dir()
        .unwrap_or_default()
        .join(".steam/steam");

    let mut cmd = Command::new(&proton_bin);
    cmd.arg("run")
       .arg(exe)
       .env("STEAM_COMPAT_DATA_PATH",          prefix)
       .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_install)
       .env("PROTON_LOG",                       "1")
       .env("DXVK_LOG_LEVEL",                   "none");

    if let Some(wd) = working_dir {
        cmd.current_dir(wd);
    }

    let child = cmd.spawn()
        .with_context(|| format!("Failed to spawn {}", proton_bin.display()))?;
    Ok(child)
}

// ── Plain Wine launcher ───────────────────────────────────────────────────────

pub async fn launch_wine(
    exe: &str,
    prefix: &Path,
    working_dir: Option<&str>,
) -> Result<tokio::process::Child> {
    info!("Launching '{}' via Wine (prefix: {})", exe, prefix.display());
    tokio::fs::create_dir_all(prefix).await?;

    let mut cmd = Command::new("wine");
    cmd.arg(exe)
       .env("WINEPREFIX", prefix);

    if let Some(wd) = working_dir {
        cmd.current_dir(wd);
    }

    cmd.spawn().context("wine not found")
}

// ── Proton GE auto-install ────────────────────────────────────────────────────

/// Download and install the latest GE-Proton release.
/// Uses the GitHub Releases API; the archive is extracted to compatibilitytools.d.
pub async fn install_ge_proton_latest() -> Result<ProtonVersion> {
    let client = reqwest::Client::builder()
        .user_agent("hackeros-hacker-mode/0.1")
        .build()?;

    // Fetch latest release info
    let release: serde_json::Value = client
        .get("https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest")
        .send().await?
        .json().await?;

    let tag = release["tag_name"].as_str().context("No tag_name in release")?;
    let assets = release["assets"].as_array().context("No assets")?;

    let tar_asset = assets.iter()
        .find(|a| a["name"].as_str().map(|n| n.ends_with(".tar.gz")).unwrap_or(false))
        .context("No .tar.gz asset")?;

    let url = tar_asset["browser_download_url"]
        .as_str().context("No download URL")?;
    let filename = tar_asset["name"].as_str().context("No filename")?;

    let tools_dir = dirs::home_dir()
        .context("No home dir")?
        .join(".steam/root/compatibilitytools.d");
    tokio::fs::create_dir_all(&tools_dir).await?;

    let tar_path = tools_dir.join(filename);
    info!("Downloading {tag} from {url}");

    // Stream download
    let mut resp = client.get(url).send().await?;
    let mut file = tokio::fs::File::create(&tar_path).await?;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    info!("Extracting {}", tar_path.display());
    let status = Command::new("tar")
        .args(["xzf", tar_path.to_str().unwrap()])
        .current_dir(&tools_dir)
        .status().await?;
    if !status.success() {
        anyhow::bail!("tar extraction failed");
    }
    tokio::fs::remove_file(&tar_path).await.ok();

    let version_path = tools_dir.join(tag.trim_start_matches('v').to_owned());
    // Tag is like "GE-Proton9-20"; the extracted dir matches
    let extracted = tools_dir.join(tag);
    let version_path = if extracted.exists() { extracted } else { version_path };

    info!("GE-Proton installed: {}", version_path.display());
    Ok(ProtonVersion { name: tag.to_owned(), path: version_path })
}
