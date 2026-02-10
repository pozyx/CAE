#include <CLI/CLI.hpp>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>

#ifdef _WIN32
#include <windows.h>
#endif

#include "config.h"
#include "render.h"

// Force NVIDIA/AMD discrete GPU on hybrid graphics (Optimus/PowerXpress) systems.
// Without this, OpenGL may run on the Intel integrated GPU while CUDA runs on the
// discrete GPU, making CUDA-GL interop impossible (cudaGraphicsGLRegisterBuffer fails).
// On Linux, use environment variable __NV_PRIME_RENDER_OFFLOAD=1 instead.
#ifdef _WIN32
extern "C" {
    __declspec(dllexport) unsigned long NvOptimusEnablement = 1;
    __declspec(dllexport) int AmdPowerXpressRequestHighPerformance = 1;
}
#endif

static void print_banner(const cae::Config& config) {
    // Format initial state display
    std::string initial_display;
    if (config.initial_state.has_value()) {
        const auto& s = config.initial_state.value();
        if (s.length() > 30) {
            initial_display = s.substr(0, 27) + "...";
        } else {
            initial_display = s;
        }
    } else {
        initial_display = "1 (single cell)";
    }

    // Helper: pad a content string to fixed width inside box borders.
    // Counts UTF-8 continuation bytes so multi-byte chars don't misalign the border.
    auto box_line = [](const std::string& content, int inner_width = 50) -> std::string {
        int extra_bytes = 0;
        for (unsigned char c : content) {
            if ((c & 0xC0) == 0x80) extra_bytes++;
        }
        std::ostringstream oss;
        oss << "\xe2\x95\x91 " << std::left << std::setw(inner_width - 2 + extra_bytes) << content << " \xe2\x95\x91";
        return oss.str();
    };

    // Box drawing characters (UTF-8)
    const char* top    = "\xe2\x95\x94"; // U+2554
    const char* mid    = "\xe2\x95\xa0"; // U+2560
    const char* bot    = "\xe2\x95\x9a"; // U+255A
    const char* horiz  = "\xe2\x95\x90"; // U+2550
    const char* tr     = "\xe2\x95\x97"; // U+2557
    const char* mr     = "\xe2\x95\xa3"; // U+2563
    const char* br     = "\xe2\x95\x9d"; // U+255D
    const char* bullet = "\xe2\x80\xa2"; // U+2022

    // Build horizontal bar
    std::string bar;
    for (int i = 0; i < 50; ++i) bar += horiz;

    std::cout << top << bar << tr << std::endl;
    std::cout << box_line("  CAE - Cellular Automaton Engine") << std::endl;
    std::cout << mid << bar << mr << std::endl;
    std::cout << box_line(std::string("Rule: ") + std::to_string(config.rule)) << std::endl;
    std::cout << box_line(std::string("Initial State: ") + initial_display) << std::endl;
    std::cout << mid << bar << mr << std::endl;
    std::cout << box_line("Controls:") << std::endl;
    std::cout << box_line(std::string(" ") + bullet + " Drag to pan (mouse or touch)") << std::endl;
    std::cout << box_line(std::string(" ") + bullet + " Scroll wheel or pinch to zoom") << std::endl;
    std::cout << box_line(std::string(" ") + bullet + " 0: Reset viewport to initial position") << std::endl;
    std::cout << box_line(std::string(" ") + bullet + " F11: Toggle fullscreen") << std::endl;
    std::cout << box_line(std::string(" ") + bullet + " ESC: Exit ") << std::endl;
    std::cout << bot << bar << br << std::endl;
    std::cout << std::endl;
}

int main(int argc, char* argv[]) {
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    // Set Per-Monitor DPI Awareness V2 before any window creation.
    // GLFW tries to do this in glfwInit(), but it can fail silently
    // (e.g. if the console subsystem already set a different mode).
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
#endif

    CLI::App app{"CAE - 1D Cellular Automaton Engine with GPU acceleration (CUDA)"};
    app.set_help_flag("-h,--help", "Print help");

    cae::Config config;
    uint16_t rule_val = 0;
    std::string initial_state_str;

    auto rule_validator = CLI::Range(static_cast<uint16_t>(0), static_cast<uint16_t>(255));
    rule_validator.description("");
    app.add_option("-r,--rule", rule_val, "Wolfram CA rule number (0-255)")
        ->required()
        ->check(rule_validator)
        ->type_name("<RULE>")
        ->option_text("");

    app.add_option("--initial-state", initial_state_str,
        "Initial state as binary string (e.g., \"00100\") [default: single center cell]")
        ->type_name("<INITIAL_STATE>");

    app.add_option("--width", config.width, "Window width in pixels")
        ->default_val(cae::constants::DEFAULT_WIDTH)
        ->type_name("<WIDTH>");

    app.add_option("--height", config.height, "Window height in pixels")
        ->default_val(cae::constants::DEFAULT_HEIGHT)
        ->type_name("<HEIGHT>");

    app.add_flag("-f,--fullscreen", config.fullscreen, "Start in fullscreen mode");

    app.add_option("--debounce-ms", config.debounce_ms,
        "Debounce time in milliseconds before recomputing after viewport change")
        ->default_val(cae::constants::DEFAULT_DEBOUNCE_MS)
        ->type_name("<DEBOUNCE_MS>");

    app.add_option("--cache-tiles", config.cache_tiles,
        "Maximum number of tiles to cache (0 to disable caching)")
        ->default_val(cae::constants::DEFAULT_CACHE_TILES)
        ->type_name("<CACHE_TILES>");

    app.add_option("--cache-tile-size", config.tile_size,
        "Cache tile size (tiles are NxN cells)")
        ->default_val(cae::constants::DEFAULT_TILE_SIZE)
        ->type_name("<CACHE_TILE_SIZE>");

    CLI11_PARSE(app, argc, argv);

    config.rule = static_cast<uint8_t>(rule_val);
    if (!initial_state_str.empty()) {
        config.initial_state = initial_state_str;
    }

    // Validate configuration
    auto errors = config.validate();
    if (!errors.empty()) {
        for (const auto& error : errors) {
            std::cerr << "Error: " << error << std::endl;
        }
        std::cerr << std::endl;
        std::cerr << "For more information, try '--help'." << std::endl;
        return 1;
    }

    print_banner(config);

    cae::RenderApp renderApp(config);
    renderApp.run();

    return 0;
}
