// Note: This is a basic example of an X11 window manager using XCB in C++. It's inspired by simple tiling WMs like flukewm.
// To make it a full compositor, you need to add the XComposite and XDamage extensions for off-screen rendering and compositing.
// For simplicity, this is a non-compositing WM, but I've added comments where compositing can be extended.
// For TOML parsing, use toml++ (header-only library): download from https://github.com/marzer/tomlplusplus and include "toml++/toml.hpp"
// Compile with: g++ -o hacker-mode hacker-mode.cpp `pkg-config --cflags --libs xcb xcb-ewmh xcb-icccm xcb-randr` -std=c++17
// To make it AppImage, use appimage-builder or similar tool.
// The config file is ~/.hackeros/Hacker-Mode/config.toml with format:
// [config]
// gap = 10
// master_factor = 60
// [apps]
// app1 = "firefox"
// app2 = "xterm"

// For future XWayland, you can port to Wayland compositor libraries like wlroots.

#include <xcb/xcb.h>
#include <xcb/xproto.h>
#include <xcb/xcb_icccm.h>
#include <xcb/xcb_ewmh.h>
#include <xcb/randr.h>
#include <toml++/toml.hpp>  // Include toml++ header

#include <cstdlib>
#include <cstring>
#include <string>
#include <iostream>
#include <vector>
#include <algorithm>
#include <unordered_map>
#include <fstream>
#include <sys/stat.h>

enum class Layout {
  Tile,
  Monocle,
  Grid
};

struct Monitor {
  xcb_window_t root;
  int x, y, width, height;
  int wx, wy, ww, wh; // usable window area
  std::vector<xcb_window_t> clients;
  xcb_window_t focused = XCB_NONE;
  Layout layout = Layout::Tile;
  int master_factor = 60;
  int master_count = 1;
  int gap = 10;
  bool show_bar = true;
};

class HackerModeWM {
public:
  HackerModeWM();
  ~HackerModeWM();
  void run();
private:
  xcb_connection_t *conn;
  xcb_screen_t *screen;
  xcb_ewmh_connection_t ewmh;
  std::vector<Monitor> monitors;
  Monitor *selmon;
  std::unordered_map<xcb_window_t, Monitor *> client_to_mon;
  std::vector<std::string> apps_to_launch;

  void load_config();
  void launch_apps();

  void scan();
  void setup();
  void cleanup();

  void handle_event(xcb_generic_event_t *evt);

  // Event handlers
  void buttonpress(xcb_generic_event_t *evt);
  void configurerequest(xcb_generic_event_t *evt);
  void configurenotify(xcb_generic_event_t *evt);
  void destroynotify(xcb_generic_event_t *evt);
  void enternotify(xcb_generic_event_t *evt);
  void focusin(xcb_generic_event_t *evt);
  void keypress(xcb_generic_event_t *evt);
  void mappingnotify(xcb_generic_event_t *evt);
  void maprequest(xcb_generic_event_t *evt);
  void motionnotify(xcb_generic_event_t *evt);
  void propertynotify(xcb_generic_event_t *evt);
  void unmapnotify(xcb_generic_event_t *evt);

  void arrange(Monitor *m);
  void tile(Monitor *m);
  void monocle(Monitor *m);
  void grid(Monitor *m);

  void attach(Monitor *m, xcb_window_t win);
  void detach(Monitor *m, xcb_window_t win);

  void focus(Monitor *m, xcb_window_t win);
  void unfocus(xcb_window_t win);

  void update_monitors();
  Monitor *get_mon_from_window(xcb_window_t win);
  Monitor *get_mon_from_point(int16_t x, int16_t y);

  void grabkeys();
  void grabbuttons(xcb_window_t win);

  xcb_keysym_t get_keysym(xcb_keycode_t keycode);

  // Add compositing setup (basic)
  void setup_compositing();
};

typedef void (HackerModeWM::*EventHandler)(xcb_generic_event_t *);

static EventHandler event_handlers[256] = {nullptr};

#define SET_HANDLER(type, handler) event_handlers[type] = &HackerModeWM::handler

HackerModeWM::HackerModeWM() {
  conn = xcb_connect(nullptr, nullptr);
  if (xcb_connection_has_error(conn)) {
    std::cerr << "Cannot open display" << std::endl;
    exit(1);
  }

  screen = xcb_setup_roots_iterator(xcb_get_setup(conn)).data;

  xcb_ewmh_init_atoms_replies(&ewmh, xcb_ewmh_init_atoms(conn, &ewmh), nullptr);

  load_config();
  setup();
  setup_compositing();  // Setup for compositing
  scan();
  launch_apps();
}

HackerModeWM::~HackerModeWM() {
  cleanup();
  xcb_disconnect(conn);
}

void HackerModeWM::run() {
  xcb_generic_event_t *evt;
  while ((evt = xcb_wait_for_event(conn))) {
    uint8_t type = evt->response_type & ~0x80;
    if (event_handlers[type]) {
      (this->*event_handlers[type])(evt);
    }
    free(evt);
  }
}

void HackerModeWM::load_config() {
  std::string home = getenv("HOME");
  std::string path = home + "/.hackeros/Hacker-Mode/config.toml";

  struct stat buffer;
  if (stat(path.c_str(), &buffer) != 0) {
    std::cerr << "Config file not found, using defaults." << std::endl;
    return;
  }

  try {
    toml::table tbl = toml::parse_file(path);

    if (auto config = tbl["config"]) {
      if (auto gap = config["gap"].value<int>()) selmon->gap = *gap;
      if (auto mf = config["master_factor"].value<int>()) selmon->master_factor = *mf;
    }

    if (auto apps = tbl["apps"].as_table()) {
      for (const auto& [key, value] : *apps) {
        if (value.is_string()) apps_to_launch.push_back(value.as_string()->get());
      }
    }
  } catch (const toml::parse_error& err) {
    std::cerr << "Parsing error: " << err << std::endl;
  }
}

void HackerModeWM::launch_apps() {
  for (const auto& app : apps_to_launch) {
    if (fork() == 0) {
      execl("/bin/sh", "sh", "-c", app.c_str(), (char *)nullptr);
      exit(0);
    }
  }
}

void HackerModeWM::setup() {
  uint32_t values[1] = {XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT | XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY | XCB_EVENT_MASK_BUTTON_PRESS | XCB_EVENT_MASK_ENTER_WINDOW | XCB_EVENT_MASK_STRUCTURE_NOTIFY | XCB_EVENT_MASK_PROPERTY_CHANGE};

  xcb_change_window_attributes_checked(conn, screen->root, XCB_CW_EVENT_MASK, values);

  xcb_flush(conn);

  grabkeys();

  update_monitors();
  selmon = &monitors[0];

  SET_HANDLER(XCB_BUTTON_PRESS, buttonpress);
  SET_HANDLER(XCB_CONFIGURE_REQUEST, configurerequest);
  SET_HANDLER(XCB_CONFIGURE_NOTIFY, configurenotify);
  SET_HANDLER(XCB_DESTROY_NOTIFY, destroynotify);
  SET_HANDLER(XCB_ENTER_NOTIFY, enternotify);
  SET_HANDLER(XCB_FOCUS_IN, focusin);
  SET_HANDLER(XCB_KEY_PRESS, keypress);
  SET_HANDLER(XCB_MAPPING_NOTIFY, mappingnotify);
  SET_HANDLER(XCB_MAP_REQUEST, maprequest);
  SET_HANDLER(XCB_MOTION_NOTIFY, motionnotify);
  SET_HANDLER(XCB_PROPERTY_NOTIFY, propertynotify);
  SET_HANDLER(XCB_UNMAP_NOTIFY, unmapnotify);
}

void HackerModeWM::setup_compositing() {
  // Check for Composite extension
  xcb_composite_query_version_reply_t *reply = xcb_composite_query_version_reply(conn, xcb_composite_query_version(conn, 0, 3), nullptr);
  if (reply && (reply->major_version > 0 || (reply->minor_version >= 3))) {
    // Enable manual redirection for compositing
    xcb_composite_redirect_subwindows(conn, screen->root, XCB_COMPOSITE_REDIRECT_MANUAL);
    // Note: To fully composite, you need to handle Damage events, create pixmaps/pictures for each window, and render them (using XRender or OpenGL).
    // This is just the setup. For a full implementation, look at picom or compton source.
  } else {
    std::cerr << "Composite extension not available or too old." << std::endl;
  }
  free(reply);
}

void HackerModeWM::cleanup() {
  for (auto &m : monitors) {
    for (auto c : m.clients) {
      xcb_unmap_window(conn, c);
      xcb_reparent_window(conn, c, screen->root, 0, 0);
    }
  }
  xcb_set_input_focus(conn, XCB_INPUT_FOCUS_POINTER_ROOT, XCB_INPUT_FOCUS_POINTER_ROOT, XCB_CURRENT_TIME);
  xcb_flush(conn);
}

void HackerModeWM::scan() {
  // Similar to the provided example, scan existing windows and map them
  xcb_query_tree_reply_t *tree = xcb_query_tree_reply(conn, xcb_query_tree(conn, screen->root), nullptr);
  if (!tree) return;

  xcb_window_t *children = xcb_query_tree_children(tree);
  int len = xcb_query_tree_children_length(tree);

  for (int i = 0; i < len; ++i) {
    xcb_get_window_attributes_reply_t *attr = xcb_get_window_attributes_reply(conn, xcb_get_window_attributes(conn, children[i]), nullptr);
    if (attr && attr->map_state == XCB_MAP_STATE_VIEWABLE) {
      maprequest((xcb_generic_event_t*)&(xcb_map_request_event_t{.window = children[i]}));
    }
    free(attr);
  }
  free(tree);
}

// Add more event handlers and functions as in the extracted code from flukewm, but abbreviated for brevity.
// For full functionality, expand with tile, monocle, grid, attach, detach, focus, etc.

int main() {
  HackerModeWM wm;
  wm.run();
  return 0;
}
