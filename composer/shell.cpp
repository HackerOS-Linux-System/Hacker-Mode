#include "shell.hpp"
#include "focus.hpp"

void handle_new_xdg_surface(wl_listener* listener, void* data) {
    wlr_xdg_surface* xdg_surface = static_cast<wlr_xdg_surface*>(data);
    if (xdg_surface->role != WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
        return;
    }

    // Create Window struct
    auto win = std::make_shared<Window>();
    win->xdg_surface = xdg_surface;
    win->surface = xdg_surface->surface;

    // Map to space, add listeners for map, unmap, destroy, request_move, etc.

    // For example:
    wl_signal_add(&xdg_surface->surface->events.commit, &surface_commit_listener);
    surface_commit_listener.notify = [](wl_listener* l, void* d) {
        // Handle commit
    };

    CompositorState* state = // get state from container
    state->windows[xdg_surface->surface] = win;
}

void handle_new_layer_surface(wl_listener* listener, void* data) {
    // Similar handling for layer surfaces
}
