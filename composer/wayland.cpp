#include "state.hpp"
#include <wlroots.hpp>

void run_wayland() {
    wlr_backend* backend = wlr_wl_backend_create(wl_display_create(), nullptr);
    if (!backend) {
        exit(1);
    }

    CompositorState state;
    init_state(state, backend);

    wlr_backend_start(backend);

    while (wl_display_run(state.display)) {}

    // Cleanup
}
