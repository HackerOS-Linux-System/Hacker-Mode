#include "cursor.hpp"

void process_cursor_motion(CompositorState& state, uint32_t time) {
    // Handle cursor movement, warp, etc.
    double cursor_x = state.cursor->x;
    double cursor_y = state.cursor->y;

    // Find surface under cursor, set focus, etc.
}
