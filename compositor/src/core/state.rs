use std::{ffi::OsString, sync::{Arc, Mutex}, time::Instant};

use smithay::{
    desktop::{PopupManager, Space, Window},
    input::{
        dnd::DndGrabHandler,
        pointer::CursorImageStatus,
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        calloop::{
            generic::Generic, EventLoop, Interest, LoopSignal,
            Mode, PostAction,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer::WlBuffer, wl_surface::WlSurface},
            Display, DisplayHandle, Resource,
        },
    },
    utils::{Logical, Point},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState,
            CompositorHandler, CompositorState,
        },
        output::{OutputHandler, OutputManagerState},
        selection::{
            data_device::{
                set_data_device_focus, DataDeviceHandler, DataDeviceState,
                WaylandDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
            XdgShellState,
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
        xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    },
};
use tracing::info;

use hm_core::{perf::PerfMonitor, settings::Config};

// ── Client state ─────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _id: ClientId) {}
    fn disconnected(&self, _id: ClientId, _reason: DisconnectReason) {}
}

// ── Compositor state ──────────────────────────────────────────────────────────

pub struct HackerModeState {
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,

    // Desktop
    pub space: Space<Window>,
    pub popups: PopupManager,

    // Smithay protocol states
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub seat: Seat<Self>,
    pub xwayland_shell_state: XWaylandShellState,

    // Input
    pub pointer_location: Point<f64, Logical>,
    pub cursor_status: Arc<Mutex<CursorImageStatus>>,

    // Submodules
    pub xwayland_state: Option<crate::xwayland::XWaylandState>,
    pub drm_outputs: Vec<crate::drm::DrmOutput>,

    // App
    pub config: Config,
    pub perf: Arc<PerfMonitor>,
    pub frame_count: u64,
    pub last_fps_time: Instant,
    pub current_fps: u32,
}

impl HackerModeState {
    pub fn new(
        event_loop: &mut EventLoop<'static, Self>,
        display: Display<Self>,
        config: Config,
    ) -> Self {
        let dh = display.handle();
        let loop_signal = event_loop.get_signal();
        let socket_name = Self::init_wayland_listener(display, event_loop);
        info!("Wayland socket: {:?}", socket_name);

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state  = XdgShellState::new::<Self>(&dh);
        let shm_state        = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let data_device_state    = DataDeviceState::new::<Self>(&dh);
        let xwayland_shell_state = XWaylandShellState::new::<Self>(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "seat0");
        seat.add_keyboard(Default::default(), 200, 25).unwrap();
        seat.add_pointer();

        Self {
            socket_name,
            display_handle: dh,
            loop_signal,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            data_device_state,
            seat_state,
            seat,
            xwayland_shell_state,
            pointer_location: Point::from((0.0, 0.0)),
            cursor_status: Arc::new(Mutex::new(CursorImageStatus::default_named())),
            xwayland_state: None,
            drm_outputs: Vec::new(),
            config,
            perf: Arc::new(PerfMonitor::new()),
            frame_count: 0,
            last_fps_time: Instant::now(),
            current_fps: 0,
        }
    }

    fn init_wayland_listener(
        display: Display<Self>,
        event_loop: &mut EventLoop<'static, Self>,
    ) -> OsString {
        let socket = ListeningSocketSource::new_auto()
            .expect("Failed to create Wayland socket");
        let name = socket.socket_name().to_os_string();
        let handle = event_loop.handle();

        handle.insert_source(socket, |stream, _, state| {
            state.display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("insert_client failed");
        }).expect("insert Wayland socket source");

        handle.insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_, display, state| {
                unsafe { display.get_mut().dispatch_clients(state).unwrap(); }
                Ok(PostAction::Continue)
            },
        ).expect("insert display source");

        name
    }

    pub fn on_idle(&mut self) {
        self.space.refresh();
        self.popups.cleanup();
        self.frame_count += 1;
        let elapsed = self.last_fps_time.elapsed();
        if elapsed.as_secs() >= 1 {
            self.current_fps =
                (self.frame_count as f64 / elapsed.as_secs_f64()) as u32;
            self.perf.update_fps(self.current_fps);
            self.frame_count = 0;
            self.last_fps_time = Instant::now();
        }
    }
}

// ── BufferHandler ─────────────────────────────────────────────────────────────

impl BufferHandler for HackerModeState {
    fn buffer_destroyed(&mut self, _buf: &WlBuffer) {}
}

// ── CompositorHandler ─────────────────────────────────────────────────────────

impl CompositorHandler for HackerModeState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }
    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(p) = get_parent(&root) { root = p; }
            if let Some(w) = self.space.elements()
                .find(|w| w.toplevel().map(|t| t.wl_surface() == &root).unwrap_or(false))
                .cloned()
            {
                w.on_commit();
            }
        }
    }
}

// ── ShmHandler ────────────────────────────────────────────────────────────────

impl ShmHandler for HackerModeState {
    fn shm_state(&self) -> &ShmState { &self.shm_state }
}

// ── XdgShellHandler ───────────────────────────────────────────────────────────

impl XdgShellHandler for HackerModeState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState { &mut self.xdg_shell_state }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
        surface.with_pending_state(|s| { s.states.set(xdg_toplevel::State::Activated); });
        surface.send_configure();
        let w = Window::new_wayland_window(surface);
        self.space.map_element(w, (0, 0), true);
    }
    fn new_popup(&mut self, _: PopupSurface, _: PositionerState) {}
    fn grab(&mut self, _: PopupSurface, _: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, _: smithay::utils::Serial) {}
    fn reposition_request(&mut self, _: PopupSurface, _: PositionerState, _: u32) {}
}

// ── SeatHandler ───────────────────────────────────────────────────────────────

impl SeatHandler for HackerModeState {
    type KeyboardFocus = WlSurface;
    type PointerFocus  = WlSurface;
    type TouchFocus    = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> { &mut self.seat_state }

    fn cursor_image(&mut self, _: &Seat<Self>, img: CursorImageStatus) {
        *self.cursor_status.lock().unwrap() = img;
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}


// ── DndGrabHandler ────────────────────────────────────────────────────────────

impl DndGrabHandler for HackerModeState {}

// ── SelectionHandler / DataDevice ─────────────────────────────────────────────

impl SelectionHandler for HackerModeState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for HackerModeState {
    fn data_device_state(&mut self) -> &mut DataDeviceState { &mut self.data_device_state }
}

impl WaylandDndGrabHandler for HackerModeState {}

// ── OutputHandler ─────────────────────────────────────────────────────────────

impl OutputHandler for HackerModeState {}

// ── XWaylandShellHandler ──────────────────────────────────────────────────────

impl XWaylandShellHandler for HackerModeState {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.xwayland_shell_state
    }
}

// ── delegate_dispatch2 — the single macro that wires everything ───────────────

smithay::delegate_dispatch2!(HackerModeState);
