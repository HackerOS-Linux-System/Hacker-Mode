#define _POSIX_C_SOURCE 200809L
#ifndef WLR_USE_UNSTABLE
#define WLR_USE_UNSTABLE
#endif

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
#include <wayland-server-core.h>
#include <wayland-server-protocol.h>
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

#include "xdg-shell-protocol.h"

/*
 * Hacker Mode Compositor (C Version)
 * Updated for wlroots 0.18
 */

struct server {
    struct wl_display *wl_display;
    struct wlr_backend *backend;
    struct wlr_renderer *renderer;
    struct wlr_allocator *allocator;
    struct wlr_scene *scene;
    struct wlr_compositor *compositor;

    struct wlr_xdg_shell *xdg_shell;
    struct wl_listener new_xdg_surface;

    struct wlr_cursor *cursor;
    struct wlr_xcursor_manager *cursor_mgr;
    struct wl_listener cursor_motion;
    struct wl_listener cursor_motion_absolute;
    struct wl_listener cursor_button;
    struct wl_listener cursor_axis;
    struct wl_listener cursor_frame;

    struct wlr_seat *seat;
    struct wl_listener new_input;
    struct wl_listener request_cursor;
    struct wl_listener request_set_selection;

    struct wlr_output_layout *output_layout;
    struct wl_list outputs; // struct output::link
    struct wl_listener new_output;

    #ifdef HAS_XWAYLAND
    struct wlr_xwayland *xwayland;
    struct wl_listener new_xwayland_surface;
    #endif
};

struct output {
    struct wl_list link;
    struct server *server;
    struct wlr_output *wlr_output;
    struct wl_listener frame;
    struct wl_listener destroy;
};

struct keyboard {
    struct wl_list link;
    struct server *server;
    struct wlr_keyboard *wlr_keyboard;
    struct wl_listener modifiers;
    struct wl_listener key;
    struct wl_listener destroy;
};

// --- Input Handling ---

static void keyboard_handle_modifiers(struct wl_listener *listener, void *data) {
    (void)data; // Unused
    struct keyboard *keyboard = wl_container_of(listener, keyboard, modifiers);
    wlr_seat_set_keyboard(keyboard->server->seat, keyboard->wlr_keyboard);
    wlr_seat_keyboard_notify_modifiers(keyboard->server->seat,
                                       &keyboard->wlr_keyboard->modifiers);
}

static bool handle_keybinding(struct server *server, xkb_keysym_t sym) {
    if (sym == XKB_KEY_Escape) {
        // Safe exit
        wl_display_terminate(server->wl_display);
        return true;
    }
    return false;
}

static void keyboard_handle_key(struct wl_listener *listener, void *data) {
    struct keyboard *keyboard = wl_container_of(listener, keyboard, key);
    struct wlr_keyboard_key_event *event = data;
    wlr_seat_set_keyboard(keyboard->server->seat, keyboard->wlr_keyboard);
    wlr_seat_keyboard_notify_key(keyboard->server->seat, event->time_msec,
                                 event->keycode, event->state);

    // Handle keybindings
    if (event->state == WL_KEYBOARD_KEY_STATE_PRESSED) {
        const xkb_keysym_t *syms;
        int nsyms = xkb_state_key_get_syms(keyboard->wlr_keyboard->xkb_state,
                                           event->keycode + 8, &syms);
        for (int i = 0; i < nsyms; i++) {
            handle_keybinding(keyboard->server, syms[i]);
        }
    }
}

static void keyboard_handle_destroy(struct wl_listener *listener, void *data) {
    (void)data; // Unused
    struct keyboard *keyboard = wl_container_of(listener, keyboard, destroy);
    wl_list_remove(&keyboard->link);
    wl_list_remove(&keyboard->modifiers.link);
    wl_list_remove(&keyboard->key.link);
    wl_list_remove(&keyboard->destroy.link);
    free(keyboard);
}

static void server_new_keyboard(struct server *server, struct wlr_input_device *device) {
    struct keyboard *keyboard = calloc(1, sizeof(struct keyboard));
    keyboard->server = server;
    keyboard->wlr_keyboard = wlr_keyboard_from_input_device(device);

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
    keyboard->destroy.notify = keyboard_handle_destroy;
    wl_signal_add(&device->events.destroy, &keyboard->destroy);

    wlr_seat_set_keyboard(server->seat, keyboard->wlr_keyboard);
}

static void server_new_pointer(struct server *server, struct wlr_input_device *device) {
    wlr_cursor_attach_input_device(server->cursor, device);
}

static void server_new_input(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, new_input);
    struct wlr_input_device *device = data;
    uint32_t caps = WL_SEAT_CAPABILITY_POINTER;

    switch (device->type) {
        case WLR_INPUT_DEVICE_KEYBOARD:
            server_new_keyboard(server, device);
            caps |= WL_SEAT_CAPABILITY_KEYBOARD;
            break;
        case WLR_INPUT_DEVICE_POINTER:
            server_new_pointer(server, device);
            // Check if we already have a keyboard to keep the capability
            // (Simplified logic: in a real compositor we'd track counts)
            caps |= WL_SEAT_CAPABILITY_KEYBOARD;
            break;
        default:
            break;
    }

    wlr_seat_set_capabilities(server->seat, caps);
}

// --- Cursor Handling ---

static void process_cursor_motion(struct server *server, uint32_t time) {
    wlr_seat_pointer_notify_motion(server->seat, time, server->cursor->x, server->cursor->y);
}

static void server_cursor_motion(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, cursor_motion);
    struct wlr_pointer_motion_event *event = data;
    wlr_cursor_move(server->cursor, &event->pointer->base, event->delta_x, event->delta_y);
    process_cursor_motion(server, event->time_msec);
}

static void server_cursor_motion_absolute(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, cursor_motion_absolute);
    struct wlr_pointer_motion_absolute_event *event = data;
    wlr_cursor_warp_absolute(server->cursor, &event->pointer->base, event->x, event->y);
    process_cursor_motion(server, event->time_msec);
}

static void server_cursor_button(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, cursor_button);
    struct wlr_pointer_button_event *event = data;
    wlr_seat_pointer_notify_button(server->seat, event->time_msec, event->button, event->state);
}

static void server_cursor_axis(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, cursor_axis);
    struct wlr_pointer_axis_event *event = data;
    wlr_seat_pointer_notify_axis(server->seat, event->time_msec, event->orientation,
                                 event->delta, event->delta_discrete, event->source, WL_POINTER_AXIS_RELATIVE_DIRECTION_IDENTICAL);
}

static void server_cursor_frame(struct wl_listener *listener, void *data) {
    (void)data;
    struct server *server = wl_container_of(listener, server, cursor_frame);
    wlr_seat_pointer_notify_frame(server->seat);
}

// --- Output/Rendering ---

static void output_frame(struct wl_listener *listener, void *data) {
    (void)data;
    struct output *output = wl_container_of(listener, output, frame);
    struct wlr_scene_output *scene_output = wlr_scene_get_scene_output(output->server->scene, output->wlr_output);

    wlr_scene_output_commit(scene_output, NULL);

    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    wlr_scene_output_send_frame_done(scene_output, &now);
}

static void output_destroy(struct wl_listener *listener, void *data) {
    (void)data;
    struct output *output = wl_container_of(listener, output, destroy);
    wl_list_remove(&output->frame.link);
    wl_list_remove(&output->destroy.link);
    wl_list_remove(&output->link);
    free(output);
}

static void server_new_output(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, new_output);
    struct wlr_output *wlr_output = data;

    wlr_output_init_render(wlr_output, server->allocator, server->renderer);

    struct output *output = calloc(1, sizeof(struct output));
    output->wlr_output = wlr_output;
    output->server = server;

    output->frame.notify = output_frame;
    wl_signal_add(&wlr_output->events.frame, &output->frame);
    output->destroy.notify = output_destroy;
    wl_signal_add(&wlr_output->events.destroy, &output->destroy);

    struct wlr_output_state state;
    wlr_output_state_init(&state);
    wlr_output_state_set_enabled(&state, true);

    if (!wl_list_empty(&wlr_output->modes)) {
        struct wlr_output_mode *mode = wlr_output_preferred_mode(wlr_output);
        if (mode) {
            wlr_output_state_set_mode(&state, mode);
        }
    }

    wlr_output_commit_state(wlr_output, &state);
    wlr_output_state_finish(&state);

    wl_list_insert(&server->outputs, &output->link);
    wlr_output_layout_add_auto(server->output_layout, wlr_output);
}

// --- Window Management ---

static void focus_surface(struct server *server, struct wlr_surface *surface) {
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

// Callback for map events if needed in future
static void xdg_toplevel_map(struct wl_listener *listener, void *data) {
    (void)listener;
    (void)data;
}

static void server_new_xdg_surface(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, new_xdg_surface);
    struct wlr_xdg_surface *xdg_surface = data;

    if (xdg_surface->role == WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
        // Add to scene graph
        wlr_scene_xdg_surface_create(&server->scene->tree, xdg_surface);

        // KIOSK MODE: Force full size of the first output
        struct wlr_output *out = wlr_output_layout_get_center_output(server->output_layout);
        if (out) {
            wlr_xdg_toplevel_set_size(xdg_surface->toplevel, out->width, out->height);
        }

        // Focus immediately
        focus_surface(server, xdg_surface->surface);

        // Example hookup for map (currently unused logic but structure preserved)
        // xdg_surface->events.map.notify = xdg_toplevel_map;
    }
}

#ifdef HAS_XWAYLAND
static void server_new_xwayland_surface(struct wl_listener *listener, void *data) {
    struct server *server = wl_container_of(listener, server, new_xwayland_surface);
    struct wlr_xwayland_surface *xsurface = data;

    wlr_scene_subsurface_tree_create(&server->scene->tree, xsurface->surface);

    // Simple auto-map
    // xsurface->events.map.notify = ...
}
#endif

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;
    wlr_log_init(WLR_DEBUG, NULL);

    struct server server = {0};
    server.wl_display = wl_display_create();
    server.backend = wlr_backend_autocreate(wl_display_get_event_loop(server.wl_display), NULL);
    if (!server.backend) {
        wlr_log(WLR_ERROR, "Failed to create backend");
        return 1;
    }

    server.renderer = wlr_renderer_autocreate(server.backend);
    wlr_renderer_init_wl_display(server.renderer, server.wl_display);
    server.allocator = wlr_allocator_autocreate(server.backend, server.renderer);

    server.scene = wlr_scene_create();
    server.output_layout = wlr_output_layout_create(server.wl_display);
    wlr_scene_attach_output_layout(server.scene, server.output_layout);

    server.compositor = wlr_compositor_create(server.wl_display, 5, server.renderer);
    wlr_subcompositor_create(server.wl_display);
    wlr_data_device_manager_create(server.wl_display);

    // XDG Shell
    server.xdg_shell = wlr_xdg_shell_create(server.wl_display, 3);
    server.new_xdg_surface.notify = server_new_xdg_surface;
    wl_signal_add(&server.xdg_shell->events.new_surface, &server.new_xdg_surface);

    // Output Layout
    wl_list_init(&server.outputs);
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
    server.xwayland = wlr_xwayland_create(server.wl_display, server.compositor, true);
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
        struct timespec ts;
        ts.tv_sec = 0;
        ts.tv_nsec = 500000000; // 500ms
        nanosleep(&ts, NULL);

        // Launch AppImage
        execl("/usr/share/HackerOS/Scripts/HackerOS-Apps/Hacker-Mode.AppImage",
              "Hacker-Mode.AppImage",
              "--no-sandbox", // Usually required for Electron AppImages in some envs
              NULL);

        // Fallback for development if AppImage missing
        execl("/bin/sh", "/bin/sh", "-c", "npm start", NULL);
        exit(0);
    }

    wl_display_run(server.wl_display);

    wl_display_destroy_clients(server.wl_display);
    wl_display_destroy(server.wl_display);
    return 0;
}
