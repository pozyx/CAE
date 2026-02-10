#version 450 core

in vec2 vTexCoords;
out vec4 fragColor;

layout(std430, binding = 0) readonly buffer CaState {
    uint ca_state[];
};

layout(std140, binding = 1) uniform RenderParams {
    uint visible_width;
    uint visible_height;
    uint simulated_width;
    uint padding_left;
    uint cell_size;
    uint window_width;
    uint window_height;
    int  viewport_offset_x;
    int  viewport_offset_y;
    int  buffer_offset_x;
    int  buffer_offset_y;
    uint _padding;
};

void main() {
    // Convert texture coordinates to pixel coordinates
    uint pixel_x = uint(vTexCoords.x * float(window_width));
    uint pixel_y = uint(vTexCoords.y * float(window_height));

    // Convert pixel coordinates to cell coordinates (in current viewport space)
    uint viewport_cell_x = pixel_x / cell_size;
    uint viewport_cell_y = pixel_y / cell_size;

    // Bounds check against current viewport dimensions
    if (viewport_cell_x >= visible_width || viewport_cell_y >= visible_height) {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    // Convert viewport cell coordinates to world coordinates
    int world_x = int(viewport_cell_x) + viewport_offset_x;
    int world_y = int(viewport_cell_y) + viewport_offset_y;

    // Convert world coordinates to buffer coordinates
    int buffer_rel_x = world_x - buffer_offset_x;
    int buffer_rel_y = world_y - buffer_offset_y;

    // Check if this cell is within the buffer's bounds
    if (buffer_rel_y < 0 || buffer_rel_y >= int(visible_height)) {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    // Map to simulated grid coordinates (accounting for padding)
    int simulated_x = buffer_rel_x + int(padding_left);

    // Check horizontal bounds in simulated space
    if (simulated_x < 0 || simulated_x >= int(simulated_width)) {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    // Get cell value from simulated grid buffer
    uint idx = uint(buffer_rel_y) * simulated_width + uint(simulated_x);
    uint cell = ca_state[idx];

    // White for alive, black for dead
    if (cell == 1u) {
        fragColor = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
