#include "render.hpp"
#include "drawing.hpp"

void render_output(CompositorState& state, wlr_output* output) {
    if (!wlr_output_attach_render(output, nullptr)) {
        return;
    }

    int width, height;
    wlr_output_effective_resolution(output, &width, &height);

    wlr_renderer_begin(state.renderer, width, height);

    // Render layers, windows, etc.
    // For each window in space
    for (auto& win : state.windows) {
        draw_window(state, win.second->surface, 1.0f, 1.0f);
    }

    // Apply game effects: HDR, scaling, etc.
    // Use wlr_renderer_scissor or custom shaders

    wlr_output_render_software_cursors(output, nullptr);
    wlr_renderer_end(state.renderer);

    wlr_output_commit(output);
}
