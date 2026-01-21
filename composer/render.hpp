#ifndef RENDER_HPP
#define RENDER_HPP

#include <wlroots.hpp>
#include "state.hpp"

void render_output(CompositorState& state, wlr_output* output);

#endif
