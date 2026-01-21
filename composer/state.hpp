#ifndef STATE_HPP
#define STATE_HPP

#include <wlroots.hpp> // wlr types
#include <wayland-server.h>
#include <vector>
#include <map>
#include <memory>

struct CompositorState {
    wl_display* display;
    wlr_backend* backend;
    wlr_renderer* renderer;
    wlr_allocator* allocator;
    wlr_compositor* compositor;
    wlr_xdg_shell* xdg_shell;
    wlr_layer_shell_v1* layer_shell;
    wlr_seat* seat;
    wlr_output_layout* output_layout;
    std::vector<wlr_output*> outputs;
    std::map<wlr_surface*, std::shared_ptr<Window>> windows; // Custom Window struct

    // Add more as needed: cursor, keyboard, etc.
    wlr_cursor* cursor;
    wlr_xcursor_manager* cursor_mgr;

    // Game effects placeholders (e.g., scaling, shaders)
    // For simplicity, use wlr_renderer for effects
};

void init_state(CompositorState& state, wlr_backend* backend);

#endif
