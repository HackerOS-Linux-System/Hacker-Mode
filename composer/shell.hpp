#ifndef SHELL_HPP
#define SHELL_HPP

#include <wlroots.hpp>
#include "state.hpp"

struct Window {
    wlr_xdg_surface* xdg_surface;
    wlr_surface* surface;
    // Position, size, etc.
    int x, y;
};

void handle_new_xdg_surface(wl_listener* listener, void* data);
void handle_new_layer_surface(wl_listener* listener, void* data);

#endif
