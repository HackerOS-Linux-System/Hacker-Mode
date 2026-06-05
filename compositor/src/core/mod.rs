pub mod state;

use anyhow::Result;
use smithay::reexports::{
    calloop::EventLoop,
    wayland_server::Display,
};
use tracing::info;

use hm_core::settings::Config;
use state::HackerModeState;

pub fn run() -> Result<()> {
    info!("Initialising compositor");

    let config = Config::load_or_default(
        &dirs::config_dir()
            .unwrap_or_default()
            .join("hackeros/hacker-mode"),
    )?;

    let mut event_loop: EventLoop<'static, HackerModeState> = EventLoop::try_new()?;
    let display: Display<HackerModeState> = Display::new()?;

    let mut state = HackerModeState::new(&mut event_loop, display, config);

    // Set WAYLAND_DISPLAY so child processes find our socket
    // SAFETY: single-threaded at this point
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };

    // Start Xwayland if enabled
    if state.config.compositor.xwayland {
        if let Err(e) = crate::xwayland::start(&mut state, &event_loop.handle()) {
            tracing::warn!("Xwayland failed to start: {e}");
        }
    }

    // Init DRM outputs
    if let Err(e) = crate::drm::init(&mut state) {
        tracing::warn!("DRM init failed: {e}");
    }

    info!("Compositor event loop running (WAYLAND_DISPLAY={:?})", state.socket_name);

    event_loop.run(None, &mut state, |state| {
        state.on_idle();
    })?;

    Ok(())
}
