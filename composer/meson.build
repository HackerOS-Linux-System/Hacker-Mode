project('hacker-compositor', 'cpp',
  version : '0.1.0',
  default_options : ['warning_level=3', 'cpp_std=c++20'])

wlroots_dep = dependency('wlroots', version: '>=0.17.0')
wayland_server_dep = dependency('wayland-server')
xkbcommon_dep = dependency('xkbcommon')
libdrm_dep = dependency('libdrm')
gbm_dep = dependency('gbm')
egl_dep = dependency('egl')
glesv2_dep = dependency('glesv2')
# Add more deps as needed: pixman, etc.

sources = [
  'main.cpp',
  'state.cpp',
  'drawing.cpp',
  'render.cpp',
  'shell.cpp',
  'cursor.cpp',
  'focus.cpp',
  'drm.cpp',
  'wayland.cpp',
  'x11.cpp',
]

executable('hacker-compositor',
  sources,
  dependencies : [wlroots_dep, wayland_server_dep, xkbcommon_dep, libdrm_dep, gbm_dep, egl_dep, glesv2_dep],
  install : true)
