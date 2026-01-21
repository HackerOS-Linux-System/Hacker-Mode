#include "state.hpp"
#include <wlroots.hpp>

void run_drm() {
    wlr_backend* backend = wlr_backend_autocreate(wl_display_create(), nullptr); // Auto uses DRM if possible
    if (!backend) {
        // Error
        exit(1);
    }

    CompositorState state;
    init_state(state, backend);

    wlr_backend_start(backend);

    setenv("WAYLAND_DISPLAY", wl_display_get_socket(state.display), 1);

    while (wl_display_run(state.display)) {
        // Loop
    }

    // Cleanup
}
