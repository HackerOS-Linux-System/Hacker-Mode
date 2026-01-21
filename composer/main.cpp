#include <iostream>
#include <string>
#include <wlroots.hpp> // Assuming wlroots C++ bindings or include wlr headers
#include "state.hpp"
#include "udev.hpp"
#include "wayland.hpp"
#include "x11.hpp"

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cout << "USAGE: hacker-compositor --backend" << std::endl;
        std::cout << "Possible backends are:" << std::endl;
        std::cout << "\t--wayland (recommended for nested in Hacker Mode)" << std::endl;
        std::cout << "\t--drm" << std::endl;
        std::cout << "\t--x11" << std::endl;
        return 1;
    }

    std::string arg = argv[1];
    if (arg == "--wayland") {
        std::cout << "Starting hacker-compositor with wayland backend (nested for games)" << std::endl;
        run_wayland();
    } else if (arg == "--drm") {
        std::cout << "Starting hacker-compositor on DRM" << std::endl;
        run_drm();
    } else if (arg == "--x11") {
        std::cout << "Starting hacker-compositor with x11 backend" << std::endl;
        run_x11();
    } else {
        std::cerr << "Unknown backend: " << arg << std::endl;
        return 1;
    }
    return 0;
}
