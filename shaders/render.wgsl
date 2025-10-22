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
    width: u32,
    height: u32,
    current_iteration: u32,
    _padding: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert texture coordinates to cell coordinates
    let x = u32(in.tex_coords.x * f32(params.width));
    let y = u32(in.tex_coords.y * f32(params.height));

    // For animated mode, only show up to current_iteration
    // For static mode, current_iteration will be set to max
    if (y > params.current_iteration) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0); // Black for uncomputed rows
    }

    // Bounds check
    if (x >= params.width || y >= params.height) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Get cell value
    let idx = y * params.width + x;
    let cell = ca_state[idx];

    // White for alive, black for dead
    if (cell == 1u) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}
