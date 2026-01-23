project('hacker-mode-compositor', ['c', 'cpp'],
  version: '1.0.0',
  default_options: [
    'cpp_std=c++17',
    'c_std=c11',
    'warning_level=2',
  ],
)

# Required for wlroots to enable unstable API features
add_project_arguments('-DWLR_USE_UNSTABLE', language: 'cpp')
add_project_arguments('-DWLR_USE_UNSTABLE', language: 'c')

wlroots = dependency('wlroots')
wayland_server = dependency('wayland-server')
wayland_client = dependency('wayland-client')
xkbcommon = dependency('xkbcommon')
libinput = dependency('libinput')
pixman = dependency('pixman-1')
wayland_protocols = dependency('wayland-protocols')

# Wayland Scanner for generating protocol headers
wayland_scanner = find_program('wayland-scanner')
wl_protocol_dir = wayland_protocols.get_variable(pkgconfig: 'pkgdatadir')

# Define protocols to generate
protocols = [
  {'name': 'xdg-shell', 'path': join_paths(wl_protocol_dir, 'stable/xdg-shell/xdg-shell.xml')},
]

# Generate protocol code
protocol_srcs = []
protocol_headers = []

foreach p : protocols
  # Generate server header
  header = custom_target(
    '@0@ server-header'.format(p['name']),
    input: p['path'],
    output: '@0@-protocol.h'.format(p['name']),
    command: [wayland_scanner, 'server-header', '@INPUT@', '@OUTPUT@'],
  )

  # Generate private code
  code = custom_target(
    '@0@ private-code'.format(p['name']),
    input: p['path'],
    output: '@0@-protocol.c'.format(p['name']),
    command: [wayland_scanner, 'private-code', '@INPUT@', '@OUTPUT@'],
  )

  protocol_srcs += code
  protocol_headers += header
endforeach

# Optional: Check for XWayland support in wlroots
has_xwayland = wlroots.get_variable(pkgconfig: 'have_xwayland', internal: 'have_xwayland') == 'true'
if has_xwayland
  add_project_arguments('-DHAS_XWAYLAND', language: 'c')
endif

executable(
  'compositor',
  # Use main.c instead of main.cpp
  ['main.c'] + protocol_srcs + protocol_headers,
  dependencies: [
    wlroots,
    wayland_server,
    wayland_client,
    xkbcommon,
    libinput,
    pixman,
  ],
  # Include the build directory to find generated headers
  include_directories: include_directories('.'),
  install: true
)
