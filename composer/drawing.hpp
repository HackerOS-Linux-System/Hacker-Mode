#ifndef DRAWING_HPP
#define DRAWING_HPP

#include <wlroots.hpp>
#include "state.hpp"

void draw_window(CompositorState& state, wlr_surface* surface, float scale, float alpha);

#endif
