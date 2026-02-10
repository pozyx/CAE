#pragma once
#include <cstdint>
#include <optional>

namespace cae {

struct Viewport {
    float offset_x = 0.0f;
    float offset_y = 0.0f;
    float zoom = 1.0f;
};

struct DragState {
    bool active = false;
    double start_x = 0.0;
    double start_y = 0.0;
    Viewport viewport_at_start;
};

struct TouchPoint {
    uint64_t id = 0;
    double x = 0.0;
    double y = 0.0;
};

struct TouchState {
    std::optional<TouchPoint> single_touch;     // Single touch for panning
    std::optional<TouchPoint> touch1;           // First finger (for pinch)
    std::optional<TouchPoint> touch2;           // Second finger (for pinch)
    std::optional<float> initial_distance;      // Distance at pinch start
    std::optional<uint32_t> initial_cell_size;  // Cell size at pinch start
};

} // namespace cae
