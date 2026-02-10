#pragma once
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace cae {

namespace constants {
    constexpr uint32_t DEFAULT_CELL_SIZE = 10;
    constexpr uint32_t DEFAULT_WIDTH = 1280;
    constexpr uint32_t DEFAULT_HEIGHT = 960;
    constexpr uint64_t DEFAULT_DEBOUNCE_MS = 0;
    constexpr size_t   DEFAULT_CACHE_TILES = 64;
    constexpr uint32_t DEFAULT_TILE_SIZE = 256;
    constexpr uint8_t  DEFAULT_RULE = 30;
    constexpr float    ZOOM_MIN = 0.1f;
    constexpr float    ZOOM_MAX = 50.0f;
    constexpr uint32_t MAX_CELLS_X = 5000;
    constexpr uint32_t MAX_CELLS_Y = 5000;
    constexpr uint32_t MIN_CELL_SIZE = 2;
    constexpr uint64_t MAX_TOTAL_CELLS = 10'000'000;
    constexpr uint32_t COMPUTE_BATCH_SIZE = 32;
    constexpr uint32_t COMPUTE_BLOCK_SIZE = 256;
    constexpr uint64_t RENDER_PARAMS_THROTTLE_MS = 16;
}

struct Config {
    uint8_t rule = constants::DEFAULT_RULE;
    std::optional<std::string> initial_state;
    uint32_t width = constants::DEFAULT_WIDTH;
    uint32_t height = constants::DEFAULT_HEIGHT;
    uint64_t debounce_ms = constants::DEFAULT_DEBOUNCE_MS;
    bool fullscreen = false;
    size_t cache_tiles = constants::DEFAULT_CACHE_TILES;
    uint32_t tile_size = constants::DEFAULT_TILE_SIZE;

    // Returns empty vector if valid, otherwise list of error strings
    std::vector<std::string> validate() const;
};

} // namespace cae
