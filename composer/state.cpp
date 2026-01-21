#include "state.hpp"
#include "drawing.hpp"
#include "render.hpp"
#include "shell.hpp"
#include "focus.hpp"
#include "cursor.hpp"

// Callback declarations
static void handle_new_output(wl_listener* listener, void* data);
static void handle_new_surface(wl_listener* listener, void* data);
// etc.

void init_state(CompositorState& state, wlr_backend* backend) {
    state.display = wl_display_create();
    state.backend = backend;

    state.renderer = wlr_renderer_autocreate(backend);
    if (!state.renderer) {
        // Error handling
        exit(1);
    }

    state.allocator = wlr_allocator_autocreate(backend, state.renderer);
    if (!state.allocator) {
        // Error
        exit(1);
    }

    state.compositor = wlr_compositor_create(state.display, 5, state.renderer);

    wlr_data_device_manager_create(state.display);

    state.output_layout = wlr_output_layout_create();

    // Listeners
    wl_signal_add(&backend->events.new_output, &new_output_listener);
    new_output_listener.notify = handle_new_output;

    state.xdg_shell = wlr_xdg_shell_create(state.display, 4);
    wl_signal_add(&state.xdg_shell->events.new_surface, &new_xdg_surface_listener);
    new_xdg_surface_listener.notify = handle_new_xdg_surface;

    state.layer_shell = wlr_layer_shell_v1_create(state.display);
    wl_signal_add(&state.layer_shell->events.new_surface, &new_layer_surface_listener);
    new_layer_surface_listener.notify = handle_new_layer_surface;

    // Seat
    state.seat = wlr_seat_create(state.display, "seat0");
    // Add keyboard, pointer handlers

    state.cursor = wlr_cursor_create();
    wlr_cursor_attach_output_layout(state.cursor, state.output_layout);

    state.cursor_mgr = wlr_xcursor_manager_create(nullptr, 24);
    wlr_xcursor_manager_load(state.cursor_mgr, 1.0);

    // More initializations: input, etc.

    // For game effects, init shaders or whatever in renderer
}
