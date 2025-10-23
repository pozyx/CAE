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
    _padding: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert texture coordinates to pixel coordinates
    let pixel_x = u32(in.tex_coords.x * f32(params.window_width));
    let pixel_y = u32(in.tex_coords.y * f32(params.window_height));

    // Convert pixel coordinates to cell coordinates
    let visible_x = pixel_x / params.cell_size;
    let visible_y = pixel_y / params.cell_size;

    // Bounds check
    if (visible_x >= params.visible_width || visible_y >= params.visible_height) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Map visible coordinates to simulated grid coordinates (accounting for padding)
    let simulated_x = visible_x + params.padding_left;
    let simulated_y = visible_y;

    // Get cell value from simulated grid buffer
    let idx = simulated_y * params.simulated_width + simulated_x;
    let cell = ca_state[idx];

    // White for alive, black for dead
    if (cell == 1u) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}
