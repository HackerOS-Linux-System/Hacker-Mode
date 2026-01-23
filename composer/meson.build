project('hacker-mode-compositor', 'cpp',
  version: '1.0.0',
  default_options: [
    'cpp_std=c++17',
    'warning_level=2',
  ],
)

wlroots = dependency('wlroots')
wayland_server = dependency('wayland-server')
wayland_client = dependency('wayland-client')
xkbcommon = dependency('xkbcommon')
libinput = dependency('libinput')
pixman = dependency('pixman-1')

# Optional: Check for XWayland support in wlroots
has_xwayland = wlroots.get_variable(pkgconfig: 'have_xwayland', internal: 'have_xwayland') == 'true'
if has_xwayland
  add_project_arguments('-DHAS_XWAYLAND', language: 'cpp')
endif

executable(
  'compositor',
  'main.cpp',
  dependencies: [
    wlroots,
    wayland_server,
    wayland_client,
    xkbcommon,
    libinput,
    pixman,
  ],
  install: true
)
