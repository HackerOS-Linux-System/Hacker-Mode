#define _POSIX_C_SOURCE 200809L
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <list>
#include <string>
#include <algorithm>
#include <unistd.h>
#include <wayland-server-core.h>
#include <wlr/backend.h>
#include <wlr/render/allocator.h>
#include <wlr/render/wlr_renderer.h>
#include <wlr/types/wlr_compositor.h>
#include <wlr/types/wlr_cursor.h>
#include <wlr/types/wlr_data_device.h>
#include <wlr/types/wlr_input_device.h>
#include <wlr/types/wlr_keyboard.h>
#include <wlr/types/wlr_output.h>
#include <wlr/types/wlr_output_layout.h>
#include <wlr/types/wlr_pointer.h>
#include <wlr/types/wlr_scene.h>
#include <wlr/types/wlr_seat.h>
#include <wlr/types/wlr_subcompositor.h>
#include <wlr/types/wlr_xcursor_manager.h>
#include <wlr/types/wlr_xdg_shell.h>
#include <wlr/util/log.h>
#include <xkbcommon/xkbcommon.h>

#ifdef HAS_XWAYLAND
#include <wlr/xwayland.h>
#endif

/* 
 * Hacker Mode Compositor
 * A minimal, robust C++ compositor designed to run a single full-screen UI 
 * and games on top of it. Inspired by tinywl and gamescope.
 */

struct Server {
    wl_display *wl_display;
    wlr_backend *backend;
    wlr_renderer *renderer;
    wlr_allocator *allocator;
    wlr_scene *scene;
    
    wlr_xdg_shell *xdg_shell;
    struct wl_listener new_xdg_surface;

    wlr_cursor *cursor;
    wlr_xcursor_manager *cursor_mgr;
    struct wl_listener cursor_motion;
    struct wl_listener cursor_motion_absolute;
    struct wl_listener cursor_button;
    struct wl_listener cursor_axis;
    struct wl_listener cursor_frame;

    wlr_seat *seat;
    struct wl_listener new_input;
    struct wl_listener request_cursor;
    struct wl_listener request_set_selection;

    wlr_output_layout *output_layout;
    std::list<struct Output*> outputs;
    struct wl_listener new_output;

#ifdef HAS_XWAYLAND
    wlr_xwayland *xwayland;
    struct wl_listener new_xwayland_surface;
#endif
};

struct Output {
    struct wl_list link;
    Server *server;
    wlr_output *wlr_output;
    struct wl_listener frame;
    struct wl_listener request_state;
    struct wl_listener destroy;
};

struct Keyboard {
    struct wl_list link;
    Server *server;
    wlr_keyboard *wlr_keyboard;
    struct wl_listener modifiers;
    struct wl_listener key;
    struct wl_listener destroy;
};

// --- Input Handling ---

static void keyboard_handle_modifiers(struct wl_listener *listener, void *data) {
    Keyboard *keyboard = wl_container_of(listener, keyboard, modifiers);
    wlr_seat_set_keyboard(keyboard->server->seat, keyboard->wlr_keyboard);
    wlr_seat_keyboard_notify_modifiers(keyboard->server->seat,
        &keyboard->wlr_keyboard->modifiers);
}

static bool handle_keybinding(Server *server, xkb_keysym_t sym) {
    // Escape hatch: Ctrl+Alt+Esc to exit compositor
    if (sym == XKB_KEY_Escape) {
        // Check modifiers in real implementation, simplifying for robustness
        // wl_display_terminate(server->wl_display); 
        // return true;
    }
    return false;
}

static void keyboard_handle_key(struct wl_listener *listener, void *data) {
    Keyboard *keyboard = wl_container_of(listener, keyboard, key);
    auto *event = static_cast<struct wlr_keyboard_key_event *>(data);
    wlr_seat_set_keyboard(keyboard->server->seat, keyboard->wlr_keyboard);
    wlr_seat_keyboard_notify_key(keyboard->server->seat, event->time_msec,
        event->keycode, event->state);
}

static void server_new_keyboard(Server *server, wlr_input_device *device) {
    auto *keyboard = new Keyboard();
    keyboard->server = server;
    keyboard->wlr_keyboard = wlr_keyboard_from_input_device(device);

    // Set default keymap (US)
    struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    struct xkb_keymap *keymap = xkb_keymap_new_from_names(context, NULL, XKB_KEYMAP_COMPILE_NO_FLAGS);
    wlr_keyboard_set_keymap(keyboard->wlr_keyboard, keymap);
    xkb_keymap_unref(keymap);
    xkb_context_unref(context);

    wlr_keyboard_set_repeat_info(keyboard->wlr_keyboard, 25, 600);

    keyboard->modifiers.notify = keyboard_handle_modifiers;
    wl_signal_add(&keyboard->wlr_keyboard->events.modifiers, &keyboard->modifiers);
    keyboard->key.notify = keyboard_handle_key;
    wl_signal_add(&keyboard->wlr_keyboard->events.key, &keyboard->key);

    wlr_seat_set_keyboard(server->seat, keyboard->wlr_keyboard);
}

static void server_new_pointer(Server *server, wlr_input_device *device) {
    wlr_cursor_attach_input_device(server->cursor, device);
}

static void server_new_input(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, new_input);
    auto *device = static_cast<struct wlr_input_device *>(data);
    
    switch (device->type) {
        case WLR_INPUT_DEVICE_KEYBOARD:
            server_new_keyboard(server, device);
            break;
        case WLR_INPUT_DEVICE_POINTER:
            server_new_pointer(server, device);
            break;
        default:
            break;
    }
    
    uint32_t caps = WL_SEAT_CAPABILITY_POINTER;
    wlr_seat_set_capabilities(server->seat, caps);
}

// --- Cursor Handling ---

static void process_cursor_motion(Server *server, uint32_t time) {
    double sx, sy;
    wlr_seat_pointer_notify_motion(server->seat, time, server->cursor->x, server->cursor->y);
}

static void server_cursor_motion(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, cursor_motion);
    auto *event = static_cast<struct wlr_pointer_motion_event *>(data);
    wlr_cursor_move(server->cursor, &event->pointer->base, event->delta_x, event->delta_y);
    process_cursor_motion(server, event->time_msec);
}

static void server_cursor_motion_absolute(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, cursor_motion_absolute);
    auto *event = static_cast<struct wlr_pointer_motion_absolute_event *>(data);
    wlr_cursor_warp_absolute(server->cursor, &event->pointer->base, event->x, event->y);
    process_cursor_motion(server, event->time_msec);
}

static void server_cursor_button(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, cursor_button);
    auto *event = static_cast<struct wlr_pointer_button_event *>(data);
    wlr_seat_pointer_notify_button(server->seat, event->time_msec, event->button, event->state);
}

static void server_cursor_axis(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, cursor_axis);
    auto *event = static_cast<struct wlr_pointer_axis_event *>(data);
    wlr_seat_pointer_notify_axis(server->seat, event->time_msec, event->orientation,
        event->delta, event->delta_discrete, event->source);
}

static void server_cursor_frame(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, cursor_frame);
    wlr_seat_pointer_notify_frame(server->seat);
}

// --- Output/Rendering ---

static void output_frame(struct wl_listener *listener, void *data) {
    Output *output = wl_container_of(listener, output, frame);
    struct wlr_scene_output *scene_output = wlr_scene_get_scene_output(output->server->scene, output->wlr_output);

    wlr_scene_output_commit(scene_output, NULL);

    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    wlr_scene_output_send_frame_done(scene_output, &now);
}

static void server_new_output(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, new_output);
    auto *wlr_output = static_cast<struct wlr_output *>(data);

    wlr_output_init_render(wlr_output, server->allocator, server->renderer);

    auto *output = new Output();
    output->wlr_output = wlr_output;
    output->server = server;
    
    output->frame.notify = output_frame;
    wl_signal_add(&wlr_output->events.frame, &output->frame);

    // Auto-configure mode (use preferred)
    if (!wl_list_empty(&wlr_output->modes)) {
        struct wlr_output_mode *mode = wlr_output_preferred_mode(wlr_output);
        wlr_output_set_mode(wlr_output, mode);
        wlr_output_enable(wlr_output, true);
        if (!wlr_output_commit(wlr_output)) {
            return;
        }
    }

    server->outputs.push_back(output);
    wlr_output_layout_add_auto(server->output_layout, wlr_output);
}

// --- Window Management (Kiosk Logic) ---

// Focus logic: simpler than a full WM, just focus whatever is new or under cursor
static void focus_view(Server *server, struct wlr_surface *surface) {
    if (!surface) return;
    struct wlr_seat *seat = server->seat;
    struct wlr_surface *prev_surface = seat->keyboard_state.focused_surface;
    
    if (prev_surface == surface) return;
    
    if (prev_surface) {
        struct wlr_xdg_surface *prev_xdg = wlr_xdg_surface_try_from_wlr_surface(prev_surface);
        if (prev_xdg && prev_xdg->role == WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
             wlr_xdg_toplevel_set_activated(prev_xdg->toplevel, false);
        }
    }

    struct wlr_xdg_surface *curr_xdg = wlr_xdg_surface_try_from_wlr_surface(surface);
    if (curr_xdg && curr_xdg->role == WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
        wlr_xdg_toplevel_set_activated(curr_xdg->toplevel, true);
    }

    wlr_seat_keyboard_notify_enter(seat, surface, NULL, 0, NULL);
}

static void server_new_xdg_surface(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, new_xdg_surface);
    auto *xdg_surface = static_cast<struct wlr_xdg_surface *>(data);

    if (xdg_surface->role == WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
        xdg_surface->toplevel->request_maximize.notify = [](struct wl_listener *, void *) {}; // Ignore/Auto
        xdg_surface->toplevel->request_fullscreen.notify = [](struct wl_listener *, void *) {}; 
        
        // Add to scene graph
        wlr_scene_xdg_surface_create(server->scene->tree, xdg_surface);
        
        // KIOSK MODE: Force full size of the first output
        struct wlr_output *out = wlr_output_layout_get_center_output(server->output_layout);
        if (out) {
            wlr_xdg_toplevel_set_size(xdg_surface->toplevel, out->width, out->height);
        }

        // Focus immediately
        focus_view(server, xdg_surface->surface);

        xdg_surface->events.map.notify = [](struct wl_listener *l, void *d) {
             // Re-focus on map if needed
        };
    }
}

#ifdef HAS_XWAYLAND
static void server_new_xwayland_surface(struct wl_listener *listener, void *data) {
    Server *server = wl_container_of(listener, server, new_xwayland_surface);
    auto *xsurface = static_cast<struct wlr_xwayland_surface *>(data);

    wlr_scene_subsurface_tree_create(server->scene->tree, xsurface->surface);
    
    // Simple focus on map
    xsurface->events.map.notify = [](struct wl_listener *l, void *d) {
        // Cast logic would go here to get server from closure if strictly needed, 
        // but for Kiosk, just letting it appear is usually enough.
    };
}
#endif

int main(int argc, char **argv) {
    wlr_log_init(WLR_DEBUG, NULL);

    Server server;
    server.wl_display = wl_display_create();
    server.backend = wlr_backend_autocreate(server.wl_display, NULL);
    server.renderer = wlr_renderer_autocreate(server.backend);
    wlr_renderer_init_wl_display(server.renderer, server.wl_display);
    server.allocator = wlr_allocator_autocreate(server.backend, server.renderer);

    server.scene = wlr_scene_create();
    wlr_scene_attach_output_layout(server.scene, server.output_layout = wlr_output_layout_create());

    wlr_compositor_create(server.wl_display, 5, server.renderer);
    wlr_subcompositor_create(server.wl_display);
    wlr_data_device_manager_create(server.wl_display);

    // XDG Shell
    server.xdg_shell = wlr_xdg_shell_create(server.wl_display, 3);
    server.new_xdg_surface.notify = server_new_xdg_surface;
    wl_signal_add(&server.xdg_shell->events.new_surface, &server.new_xdg_surface);

    // Output Layout
    server.new_output.notify = server_new_output;
    wl_signal_add(&server.backend->events.new_output, &server.new_output);

    // Inputs
    server.seat = wlr_seat_create(server.wl_display, "seat0");
    server.new_input.notify = server_new_input;
    wl_signal_add(&server.backend->events.new_input, &server.new_input);

    // Cursor
    server.cursor = wlr_cursor_create();
    wlr_cursor_attach_output_layout(server.cursor, server.output_layout);
    server.cursor_mgr = wlr_xcursor_manager_create(NULL, 24);
    wlr_xcursor_manager_load(server.cursor_mgr, 1);
    
    server.cursor_motion.notify = server_cursor_motion;
    wl_signal_add(&server.cursor->events.motion, &server.cursor_motion);
    server.cursor_motion_absolute.notify = server_cursor_motion_absolute;
    wl_signal_add(&server.cursor->events.motion_absolute, &server.cursor_motion_absolute);
    server.cursor_button.notify = server_cursor_button;
    wl_signal_add(&server.cursor->events.button, &server.cursor_button);
    server.cursor_axis.notify = server_cursor_axis;
    wl_signal_add(&server.cursor->events.axis, &server.cursor_axis);
    server.cursor_frame.notify = server_cursor_frame;
    wl_signal_add(&server.cursor->events.frame, &server.cursor_frame);

#ifdef HAS_XWAYLAND
    // Initialize XWayland (Required for games and many apps)
    server.xwayland = wlr_xwayland_create(server.wl_display, server.allocator, true);
    if (server.xwayland) {
        server.new_xwayland_surface.notify = server_new_xwayland_surface;
        wl_signal_add(&server.xwayland->events.new_surface, &server.new_xwayland_surface);
        setenv("DISPLAY", server.xwayland->display_name, 1);
    } else {
        wlr_log(WLR_ERROR, "Failed to start XWayland");
    }
#endif

    const char *socket = wl_display_add_socket_auto(server.wl_display);
    if (!socket) {
        wlr_backend_destroy(server.backend);
        return 1;
    }

    if (!wlr_backend_start(server.backend)) {
        wlr_backend_destroy(server.backend);
        wl_display_destroy(server.wl_display);
        return 1;
    }

    setenv("WAYLAND_DISPLAY", socket, 1);
    wlr_log(WLR_INFO, "Running Wayland Compositor on %s", socket);

    // AUTOSTART HACKER MODE APP
    if (fork() == 0) {
        // Wait briefly for socket
        usleep(500000); 
        // Execute the npm start script or the compiled binary
        // Assuming 'hacker-mode' is in PATH or referencing the AppImage
        // For development:
        execl("/bin/sh", "/bin/sh", "-c", "npm start", NULL);
    }

    wl_display_run(server.wl_display);

    wl_display_destroy_clients(server.wl_display);
    wl_display_destroy(server.wl_display);
    return 0;
}
