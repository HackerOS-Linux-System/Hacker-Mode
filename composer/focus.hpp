#ifndef FOCUS_HPP
#define FOCUS_HPP

#include <wlroots.hpp>
#include "state.hpp"

void focus_window(CompositorState& state, std::shared_ptr<Window> win);

#endif
