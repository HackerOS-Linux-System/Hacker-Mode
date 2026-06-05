use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::info;
use zbus::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerAction {
    Shutdown,
    Reboot,
    Suspend,
    Hibernate,
    HybridSleep,
    Logout,
    SwitchToDesktop(DesktopEnvironment),
    RestartHackerMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Xfce,
    KDE,
    GNOME,
    Cosmic,
    Other(String),
}

impl DesktopEnvironment {
    pub fn label(&self) -> &str {
        match self {
            DesktopEnvironment::Xfce    => "XFCE",
            DesktopEnvironment::KDE     => "KDE Plasma",
            DesktopEnvironment::GNOME   => "GNOME",
            DesktopEnvironment::Cosmic  => "COSMIC",
            DesktopEnvironment::Other(s) => s.as_str(),
        }
    }

    /// Returns Some(session_cmd) if the DE is installed.
    fn session_command(&self) -> Option<&'static str> {
        match self {
            DesktopEnvironment::Xfce   => Some("startxfce4"),
            DesktopEnvironment::KDE    => Some("startplasma-wayland"),
            DesktopEnvironment::GNOME  => Some("gnome-session"),
            DesktopEnvironment::Cosmic => Some("cosmic-session"),
            DesktopEnvironment::Other(_) => None,
        }
    }
}

/// Detect which DEs are installed on this system.
pub fn detect_available_des() -> Vec<DesktopEnvironment> {
    let candidates = [
        ("startxfce4",         DesktopEnvironment::Xfce),
        ("startplasma-wayland",DesktopEnvironment::KDE),
        ("gnome-session",      DesktopEnvironment::GNOME),
        ("cosmic-session",     DesktopEnvironment::Cosmic),
    ];
    candidates.iter()
        .filter(|(cmd, _)| which::which(cmd).is_ok())
        .map(|(_, de)| de.clone())
        .collect()
}

// ── systemd D-Bus helpers ─────────────────────────────────────────────────────

async fn logind_call(method: &str, interactive: bool) -> Result<()> {
    let conn = Connection::system().await?;
    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        method,
        &(interactive,),
    )
    .await
    .with_context(|| format!("systemd-logind {method} failed"))?;
    Ok(())
}

// ── Execute ───────────────────────────────────────────────────────────────────

pub async fn execute(action: PowerAction) -> Result<()> {
    match action {
        PowerAction::Shutdown => {
            info!("Power action: Shutdown");
            logind_call("PowerOff", false).await
        }
        PowerAction::Reboot => {
            info!("Power action: Reboot");
            logind_call("Reboot", false).await
        }
        PowerAction::Suspend => {
            info!("Power action: Suspend");
            logind_call("Suspend", false).await
        }
        PowerAction::Hibernate => {
            info!("Power action: Hibernate");
            logind_call("Hibernate", false).await
        }
        PowerAction::HybridSleep => {
            info!("Power action: HybridSleep");
            logind_call("HybridSleep", false).await
        }
        PowerAction::Logout => {
            info!("Power action: Logout");
            // Kill the current Hacker Mode session
            Command::new("loginctl")
                .args(["terminate-session", "self"])
                .status().await?;
            Ok(())
        }
        PowerAction::RestartHackerMode => {
            info!("Power action: RestartHackerMode");
            Command::new("systemctl")
                .args(["--user", "restart", "hackeros-hm.service"])
                .status().await?;
            Ok(())
        }
        PowerAction::SwitchToDesktop(de) => {
            let cmd = de.session_command()
                .with_context(|| format!("No session command for {:?}", de))?;
            info!("Switching to DE: {} ({})", de.label(), cmd);
            // Stop Hacker Mode compositor, then exec the DE session
            Command::new("systemctl")
                .args(["--user", "stop", "hackeros-hm.service"])
                .status().await.ok();
            // Launch the DE; it will take over the TTY/display
            Command::new(cmd)
                .stdin(Stdio::null())
                .spawn()?;
            Ok(())
        }
    }
}
