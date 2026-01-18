use std::{
    sync::{atomic::Ordering, Mutex},
    time::Duration,
};

use crate::{
    drawing::*,
    render::*,
    state::{AnvilState, Backend},
};

use smithay::{
    backend::{
        allocator::{
            dmabuf::{Dmabuf, DmabufAllocator},
            gbm::{GbmAllocator, GbmBufferFlags},
            vulkan::{ImageUsageFlags, VulkanAllocator},
        },
        egl::{EGLContext, EGLDisplay},
        renderer::{
            damage::OutputDamageTracker, element::AsRenderElements, gles::GlesRenderer, Bind, ImportDma,
            ImportMemWl,
        },
        vulkan::{version::Version, Instance, PhysicalDevice},
        x11::{WindowBuilder, X11Backend, X11Event, X11Surface},
    },
    delegate_dmabuf,
    input::{
        keyboard::LedState,
        pointer::{CursorImageAttributes, CursorImageStatus},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        ash::ext,
        calloop::EventLoop,
        gbm,
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{protocol::wl_surface, Display},
    },
    utils::{DeviceFd, IsAlive, Scale},
    wayland::{
        compositor,
        dmabuf::{
            DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier,
        },
        presentation::Refresh,
    },
};
use tracing::{error, info, trace, warn};

pub const OUTPUT_NAME: &str = "x11";

#[derive(Debug)]
pub struct X11Data {
    render: bool,
    mode: Mode,
    renderer: GlesRenderer,
    damage_tracker: OutputDamageTracker,
    surface: X11Surface,
    dmabuf_state: DmabufState,
    _dmabuf_global: DmabufGlobal,
    _dmabuf_default_feedback: DmabufFeedback,
}

impl Backend for X11Data {
    fn seat_name(&self) -> String {
        "x11".to_string()
    }
}

impl DmabufHandler for AnvilState<X11Data> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.backend_data.dmabuf_state
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        if self.backend_data.renderer.bind(dmabuf).is_ok() {
            notifier.successful::<Self>();
        } else {
            notifier.failed();
        }
    }
}
delegate_dmabuf!(AnvilState<X11Data>);

pub fn run_x11() {
    // Setup X11 backend, use Vulkan or GL renderer.
    // Suitable for XWayland support.
    unimplemented!();
}
