#include "focus.hpp"

void focus_window(CompositorState& state, std::shared_ptr<Window> win) {
    if (!win) return;

    wlr_keyboard* keyboard = wlr_seat_get_keyboard(state.seat);
    if (keyboard) {
        wlr_seat_keyboard_notify_enter(state.seat, win->surface, keyboard->keycodes, keyboard->num_keycodes, &keyboard->modifiers);
    }
}
