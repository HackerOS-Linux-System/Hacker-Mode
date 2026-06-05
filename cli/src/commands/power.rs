use anyhow::Result;

pub async fn run(action: &str) -> Result<()> {
    use hm_core::settings::power::{execute, PowerAction, DesktopEnvironment};
    let pa = match action {
        "shutdown"     => PowerAction::Shutdown,
        "reboot"       => PowerAction::Reboot,
        "suspend"      => PowerAction::Suspend,
        "hibernate"    => PowerAction::Hibernate,
        "logout"       => PowerAction::Logout,
        "restart_hm"   => PowerAction::RestartHackerMode,
        s if s.starts_with("switch_to:") => {
            let de = match &s["switch_to:".len()..] {
                "xfce"   => DesktopEnvironment::Xfce,
                "kde"    => DesktopEnvironment::KDE,
                "gnome"  => DesktopEnvironment::GNOME,
                "cosmic" => DesktopEnvironment::Cosmic,
                other    => DesktopEnvironment::Other(other.to_owned()),
            };
            PowerAction::SwitchToDesktop(de)
        }
        _ => anyhow::bail!("Unknown action. Valid: shutdown|reboot|suspend|hibernate|logout|restart_hm|switch_to:<de>"),
    };
    execute(pa).await?;
    Ok(())
}
