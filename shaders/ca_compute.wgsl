// Cellular Automaton Compute Shader
// Applies a 1D CA rule to compute the next generation

@group(0) @binding(0) var<storage, read_write> state: array<u32>;
@group(0) @binding(1) var<uniform> params: Params;

struct Params {
    width: u32,
    rule: u32,
    current_row: u32,
    _padding: u32,
}

// Get cell value at position in current row
fn get_cell(x: i32) -> u32 {
    let width = i32(params.width);

    // Boundary condition: treat out-of-bounds as dead (0)
    if (x < 0 || x >= width) {
        return 0u;
    }

    let idx = params.current_row * params.width + u32(x);
    return state[idx];
}

// Apply CA rule to determine next state
fn apply_rule(left: u32, center: u32, right: u32) -> u32 {
    // Create 3-bit pattern from neighborhood
    let pattern = (left << 2u) | (center << 1u) | right;

    // Extract bit from rule number corresponding to this pattern
    // Rule bits are indexed by pattern value (0-7)
    let bit_mask = 1u << pattern;
    return (params.rule & bit_mask) >> pattern;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let width = i32(params.width);

    // Only process cells within grid width
    if (x >= width) {
        return;
    }

    // Get neighborhood
    let left = get_cell(x - 1);
    let center = get_cell(x);
    let right = get_cell(x + 1);

    // Apply rule and write to next row
    let next_state = apply_rule(left, center, right);
    let next_row = params.current_row + 1u;
    let out_idx = next_row * params.width + u32(x);

    state[out_idx] = next_state;
}
