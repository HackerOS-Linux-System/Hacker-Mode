use anyhow::{Context, Result};
use smithay::{
    wayland::selection::SelectionTarget,
    desktop::Window,
    reexports::calloop::LoopHandle,
    utils::Logical,
    xwayland::{
        X11Surface, X11Wm, XWayland, XWaylandEvent,
        XwmHandler,
        xwm::{XwmId, Reorder, ResizeEdge},
    },
};
use tracing::{error, info};

use crate::core::state::HackerModeState;

pub struct XWaylandState {
    pub wm: Option<X11Wm>,
}

pub fn start(
    state: &mut HackerModeState,
    loop_handle: &LoopHandle<'static, HackerModeState>,
) -> Result<()> {
    info!("Starting Xwayland");

    let (xwayland, client) = XWayland::spawn(
        &state.display_handle,
        None,                           // display number: auto
        std::iter::empty::<(&str,&str)>(),
        true,                           // open abstract socket
        std::process::Stdio::null(),
        std::process::Stdio::null(),
        |_| {},
    )
    .context("Failed to spawn Xwayland")?;

    let lh = loop_handle.clone();
    let dh = state.display_handle.clone();

    loop_handle
        .insert_source(xwayland, move |event, _, state| {
            match event {
                XWaylandEvent::Ready { x11_socket, display_number } => {
                    info!("Xwayland ready on :{display_number}");
                    unsafe { std::env::set_var("DISPLAY", format!(":{display_number}")); }

                    match X11Wm::start_wm(lh.clone(), &dh, x11_socket, client.clone()) {
                        Ok(wm) => {
                            if let Some(ref mut xw) = state.xwayland_state {
                                xw.wm = Some(wm);
                            }
                        }
                        Err(e) => error!("Failed to start X11 WM: {e}"),
                    }
                }
                XWaylandEvent::Error => {
                    error!("Xwayland crashed");
                    state.xwayland_state = None;
                }
            }
        })
        .context("Failed to insert Xwayland source")?;

    state.xwayland_state = Some(XWaylandState { wm: None });
    Ok(())
}

// ── XwmHandler ───────────────────────────────────────────────────────────────

impl XwmHandler for HackerModeState {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwayland_state.as_mut().unwrap().wm.as_mut().unwrap()
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}
    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}
    fn destroyed_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_mapped(true);
        let geo = window.geometry();
        let w = Window::new_x11_window(window);
        self.space.map_element(w, (geo.loc.x, geo.loc.y), true);
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let geo = window.geometry();
        let w = Window::new_x11_window(window);
        self.space.map_element(w, (geo.loc.x, geo.loc.y), false);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let found = self.space.elements()
            .find(|w| w.x11_surface().map(|s| s == &window).unwrap_or(false))
            .cloned();
        if let Some(w) = found { self.space.unmap_elem(&w); }
    }

    fn configure_request(
        &mut self, _xwm: XwmId, window: X11Surface,
        x: Option<i32>, y: Option<i32>, w: Option<u32>, h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let mut geo = window.geometry();
        if let Some(v) = x { geo.loc.x  = v; }
        if let Some(v) = y { geo.loc.y  = v; }
        if let Some(v) = w { geo.size.w = v as i32; }
        if let Some(v) = h { geo.size.h = v as i32; }
        let _ = window.configure(geo);
    }

    fn configure_notify(
        &mut self, _xwm: XwmId, _window: X11Surface,
        _geometry: smithay::utils::Rectangle<i32, Logical>,
        _above: Option<u32>,
    ) {}

    fn resize_request(&mut self, _xwm: XwmId, _: X11Surface, _: u32, _: ResizeEdge) {}
    fn move_request(&mut self, _xwm: XwmId, _: X11Surface, _: u32) {}

    fn fullscreen_request(&mut self, _xwm: XwmId, w: X11Surface) { let _ = w.set_fullscreen(true); }
    fn unfullscreen_request(&mut self, _xwm: XwmId, w: X11Surface) { let _ = w.set_fullscreen(false); }
    fn maximize_request(&mut self, _xwm: XwmId, w: X11Surface) { let _ = w.set_maximized(true); }
    fn unmaximize_request(&mut self, _xwm: XwmId, w: X11Surface) { let _ = w.set_maximized(false); }
    fn minimize_request(&mut self, _xwm: XwmId, _: X11Surface) {}
    fn unminimize_request(&mut self, _xwm: XwmId, _: X11Surface) {}

    fn allow_selection_access(&mut self, _xwm: XwmId, _: SelectionTarget) -> bool { true }

    fn send_selection(
        &mut self, _xwm: XwmId, _: SelectionTarget,
        _mime_type: String, _fd: std::os::unix::io::OwnedFd,
    ) {}
}
