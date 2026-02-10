#include "config.h"

namespace cae {

std::vector<std::string> Config::validate() const {
    std::vector<std::string> errors;

    // Rule is uint8_t, always valid (0-255)

    // Initial state: must be empty or contain only 0s and 1s
    if (initial_state.has_value()) {
        const auto& state = initial_state.value();
        if (!state.empty()) {
            for (char c : state) {
                if (c != '0' && c != '1') {
                    errors.push_back("initial_state must be empty or contain only 0s and 1s");
                    break;
                }
            }
        }
    }

    // Window width: 500-8192 pixels
    if (width < 500) {
        errors.push_back("width must be at least 500 (got " + std::to_string(width) + ")");
    }
    if (width > 8192) {
        errors.push_back("width must be at most 8192 (got " + std::to_string(width) + ")");
    }

    // Window height: 500-8192 pixels
    if (height < 500) {
        errors.push_back("height must be at least 500 (got " + std::to_string(height) + ")");
    }
    if (height > 8192) {
        errors.push_back("height must be at most 8192 (got " + std::to_string(height) + ")");
    }

    // Cache tiles: 0-256
    if (cache_tiles > 256) {
        errors.push_back("cache_tiles must be at most 256 (got " + std::to_string(cache_tiles) + ")");
    }

    // Tile size: 64-1024
    if (tile_size < 64) {
        errors.push_back("tile_size must be at least 64 (got " + std::to_string(tile_size) + ")");
    }
    if (tile_size > 1024) {
        errors.push_back("tile_size must be at most 1024 (got " + std::to_string(tile_size) + ")");
    }

    // Debounce: 0-5000ms
    if (debounce_ms > 5000) {
        errors.push_back("debounce_ms must be at most 5000 (got " + std::to_string(debounce_ms) + ")");
    }

    return errors;
}

} // namespace cae
