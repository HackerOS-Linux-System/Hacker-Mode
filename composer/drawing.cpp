#include "drawing.hpp"

void draw_window(CompositorState& state, wlr_surface* surface, float scale, float alpha) {
    // Use wlr_renderer to draw with effects
    wlr_renderer_begin(state.renderer, surface->buffer_width, surface->buffer_height);
    // Clear, draw surface tree with scaling, blur, etc.
    float clear_color[4] = {0.0f, 0.0f, 0.0f, 1.0f};
    wlr_renderer_clear(state.renderer, clear_color);

    // Render surface with custom effects (e.g., shaders for upscaling like FSR)
    // For simplicity, just render the surface
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    wlr_surface_send_frame_done(surface, &now);

    wlr_renderer_end(state.renderer);
}
