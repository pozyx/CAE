// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Fragment shader
@group(0) @binding(0) var<storage, read> ca_state: array<u32>;
@group(0) @binding(1) var<uniform> params: RenderParams;

struct RenderParams {
    visible_width: u32,
    visible_height: u32,
    simulated_width: u32,
    padding_left: u32,
    cell_size: u32,
    window_width: u32,
    window_height: u32,
    viewport_offset_x: i32,  // Current viewport offset in cells
    viewport_offset_y: i32,
    buffer_offset_x: i32,    // Offset the buffer was computed for
    buffer_offset_y: i32,
    _padding: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert texture coordinates to pixel coordinates
    let pixel_x = u32(in.tex_coords.x * f32(params.window_width));
    let pixel_y = u32(in.tex_coords.y * f32(params.window_height));

    // Convert pixel coordinates to cell coordinates (in current viewport space)
    let viewport_cell_x = pixel_x / params.cell_size;
    let viewport_cell_y = pixel_y / params.cell_size;

    // Bounds check against current viewport dimensions
    if (viewport_cell_x >= params.visible_width || viewport_cell_y >= params.visible_height) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Convert viewport cell coordinates to world coordinates
    let world_x = i32(viewport_cell_x) + params.viewport_offset_x;
    let world_y = i32(viewport_cell_y) + params.viewport_offset_y;

    // Convert world coordinates to buffer coordinates
    let buffer_rel_x = world_x - params.buffer_offset_x;
    let buffer_rel_y = world_y - params.buffer_offset_y;

    // Check if this cell is within the buffer's bounds
    // Buffer y must be in [0, visible_height), x must be in [0, simulated_width)
    if (buffer_rel_y < 0 || buffer_rel_y >= i32(params.visible_height)) {
        // Outside buffer's vertical range - show black
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Map to simulated grid coordinates (accounting for padding)
    let simulated_x = buffer_rel_x + i32(params.padding_left);

    // Check horizontal bounds in simulated space
    if (simulated_x < 0 || simulated_x >= i32(params.simulated_width)) {
        // Outside buffer's horizontal range - show black
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Get cell value from simulated grid buffer
    let idx = u32(buffer_rel_y) * params.simulated_width + u32(simulated_x);
    let cell = ca_state[idx];

    // White for alive, black for dead
    if (cell == 1u) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}
