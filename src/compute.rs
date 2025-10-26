use wgpu::util::DeviceExt;
use crate::cache::{Tile, TileKey, TileCache};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    width: u32,
    height: u32,
    rule: u32,
    current_row: u32,
}

pub struct CaResult {
    pub buffer: wgpu::Buffer,
    pub simulated_width: u32,
    pub visible_width: u32,
    pub height: u32,
    pub padding_left: u32,
}

/// Compute a single tile from generation 0 to tile_size
/// Tiles are tile_size x tile_size regions identified by grid coordinates (tile_x, tile_y)
fn compute_tile(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rule: u8,
    tile_x: i32,
    tile_y: i32,
    tile_size: u32,
    initial_state: &Option<String>,
) -> Tile {
    let tile_width = tile_size;
    let tile_height = tile_size;

    // Calculate world-space horizontal range for this tile
    let tile_start_x = tile_x * tile_width as i32;
    let tile_end_x = tile_start_x + tile_width as i32;

    // Calculate generation range for this tile
    let generation_start = tile_y * tile_height as i32;
    let generation_end = generation_start + tile_height as i32;

    println!("Computing tile ({}, {}): cells {}..{}, generations {}..{}",
        tile_x, tile_y, tile_start_x, tile_end_x, generation_start, generation_end);

    // Add padding for boundary simulation
    // Pattern can expand by generation_end cells in each direction
    let padding = generation_end.max(0) as u32;
    let simulated_width = tile_width + 2 * padding;

    // Compute from generation 0 to generation_end (includes all previous generations)
    let total_generations = generation_end.max(0) as u32;
    let buffer_height = total_generations + 1;

    // Initialize first row (generation 0) with padding
    let mut initial_row = vec![0u32; simulated_width as usize];

    if let Some(ref state_str) = initial_state {
        // Parse user-provided initial state
        // World cell W maps to buffer index: padding + (W - tile_start_x)
        // Initial state (centered at world 0) starts at: padding - tile_start_x
        let base_offset = padding as i32 - tile_start_x;
        for (i, ch) in state_str.chars().enumerate() {
            let pos = base_offset + i as i32;
            if pos >= 0 && (pos as usize) < simulated_width as usize {
                initial_row[pos as usize] = if ch == '1' { 1 } else { 0 };
            }
        }
    } else {
        // Default: single cell at world position 0
        let world_zero_in_buffer = padding as i32 - tile_start_x;
        if world_zero_in_buffer >= 0 && (world_zero_in_buffer as usize) < simulated_width as usize {
            initial_row[world_zero_in_buffer as usize] = 1;
        }
    }

    // Create buffer for all iterations from gen 0 to generation_end
    let total_cells = simulated_width * buffer_height;
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..simulated_width as usize].copy_from_slice(&initial_row);

    let ca_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Tile CA State Buffer"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    // Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("CA Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ca_compute.wgsl").into()),
    });

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("CA Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Create compute pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("CA Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("CA Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Create command encoder and dispatch all iterations
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Tile Compute Encoder"),
    });

    let workgroups = (simulated_width + 255) / 256;

    for iter in 0..total_generations {
        let params = Params {
            width: simulated_width,
            height: buffer_height,
            rule: rule as u32,
            current_row: iter,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CA Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ca_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tile Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    queue.submit(Some(encoder.finish()));

    // Extract only the tile's generation range (tile_y * 256 to (tile_y+1) * 256)
    let tile_gen_offset = (tile_y * tile_height as i32).max(0) as u32;
    let tile_buffer_size = (simulated_width * tile_height * 4) as wgpu::BufferAddress;

    let tile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Tile Output Buffer"),
        size: tile_buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Tile Copy Encoder"),
    });

    let source_offset = (tile_gen_offset * simulated_width * 4) as wgpu::BufferAddress;
    copy_encoder.copy_buffer_to_buffer(
        &ca_buffer,
        source_offset,
        &tile_buffer,
        0,
        tile_buffer_size,
    );

    queue.submit(Some(copy_encoder.finish()));

    Tile {
        buffer: tile_buffer,
        simulated_width,
        padding_left: padding,
    }
}

/// Compute CA using tile-based caching
pub fn run_ca_with_cache(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rule: u8,
    start_generation: u32,
    iterations: u32,
    visible_width: u32,
    horizontal_offset: i32,
    initial_state: Option<String>,
    cache: &mut TileCache,
) -> CaResult {
    println!("\n=== run_ca_with_cache: gen {}..{}, offset_x={}, width={} ===",
        start_generation, start_generation + iterations, horizontal_offset, visible_width);

    // Calculate world-space bounds of the visible viewport
    let viewport_x_start = horizontal_offset;
    let viewport_x_end = horizontal_offset + visible_width as i32;
    let viewport_y_start = start_generation as i32;
    let viewport_y_end = (start_generation + iterations) as i32;

    // Determine which tiles we need
    let tile_size = cache.tile_size as i32;
    let tile_x_start = viewport_x_start.div_euclid(tile_size);
    let tile_x_end = (viewport_x_end - 1).div_euclid(tile_size);
    let tile_y_start = viewport_y_start.div_euclid(tile_size);
    let tile_y_end = (viewport_y_end - 1).div_euclid(tile_size);

    println!("Viewport needs tiles: X={}..{}, Y={}..{}",
        tile_x_start, tile_x_end, tile_y_start, tile_y_end);

    // Fetch or compute all required tiles
    // First, check which tiles we have and compute missing ones
    for tile_y in tile_y_start..=tile_y_end {
        for tile_x in tile_x_start..=tile_x_end {
            let tile_key = TileKey::new(rule, &initial_state, tile_x, tile_y);

            // Check if tile exists in cache
            if cache.get(&tile_key).is_none() {
                // Cache miss - compute new tile and insert
                println!("Computing new tile ({}, {})", tile_x, tile_y);
                let new_tile = compute_tile(device, queue, rule, tile_x, tile_y, cache.tile_size, &initial_state);
                cache.insert(tile_key, new_tile);
            } else {
                println!("Using cached tile ({}, {})", tile_x, tile_y);
            }
        }
    }

    // Now assemble tiles into a single output buffer for the viewport
    // Calculate output dimensions (viewport range with padding)
    let total_generations = start_generation + iterations;
    let padding = total_generations;
    let simulated_width = visible_width + 2 * padding;
    let output_height = iterations + 1;

    println!("Output buffer: width={}, height={}, padding={}",
        simulated_width, output_height, padding);

    // Create output buffer
    let output_buffer_size = (simulated_width * output_height * 4) as wgpu::BufferAddress;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Assembled Viewport Buffer"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Copy relevant regions from tiles to output buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Tile Assembly Encoder"),
    });

    // Assemble tiles one at a time (avoiding multiple borrows)
    for tile_y in tile_y_start..=tile_y_end {
        for tile_x in tile_x_start..=tile_x_end {
            let tile_key = TileKey::new(rule, &initial_state, tile_x, tile_y);
            let tile = cache.get(&tile_key).expect("Tile should be in cache");

            // Calculate overlap between tile and viewport
            let tile_world_x_start = tile_x * tile_size;
            let tile_world_x_end = tile_world_x_start + tile_size;
            let tile_gen_start = tile_y * tile_size;
            let tile_gen_end = tile_gen_start + tile_size;

            // Find intersection with viewport
            let copy_x_start = viewport_x_start.max(tile_world_x_start);
            let copy_x_end = viewport_x_end.min(tile_world_x_end);
            let copy_gen_start = viewport_y_start.max(tile_gen_start);
            let copy_gen_end = viewport_y_end.min(tile_gen_end);

            if copy_x_end <= copy_x_start || copy_gen_end <= copy_gen_start {
                continue; // No overlap
            }

            // Map to buffer coordinates
            // Tile buffer: has padding on left = tile's padding_left
            // Output buffer: has padding on left = our padding

            // For each generation row in the overlap
            for gen in copy_gen_start..copy_gen_end {
                let gen_in_viewport = (gen - viewport_y_start) as u32;
                let gen_in_tile = (gen - tile_gen_start) as u32;

                // Calculate horizontal slice
                let slice_world_start = copy_x_start;
                let slice_world_end = copy_x_end;
                let slice_width = (slice_world_end - slice_world_start) as u32;

                // Position in tile buffer (with tile's padding)
                let x_in_tile_buffer = (slice_world_start - tile_world_x_start) as u32 + tile.padding_left;

                // Position in output buffer (with our padding)
                let x_in_output_buffer = (slice_world_start - viewport_x_start) as u32 + padding;

                // Safety checks to prevent buffer overruns
                if gen_in_tile >= tile_size as u32 || gen_in_viewport >= iterations {
                    eprintln!("Warning: Generation out of bounds (tile: {}, viewport: {})", gen_in_tile, gen_in_viewport);
                    continue;
                }

                if x_in_tile_buffer + slice_width > tile.simulated_width {
                    eprintln!("Warning: Tile buffer x overflow ({} + {} > {})",
                        x_in_tile_buffer, slice_width, tile.simulated_width);
                    continue;
                }

                if x_in_output_buffer + slice_width > simulated_width {
                    eprintln!("Warning: Output buffer x overflow ({} + {} > {})",
                        x_in_output_buffer, slice_width, simulated_width);
                    continue;
                }

                let src_offset = ((gen_in_tile * tile.simulated_width + x_in_tile_buffer) * 4) as wgpu::BufferAddress;
                let dst_offset = ((gen_in_viewport * simulated_width + x_in_output_buffer) * 4) as wgpu::BufferAddress;
                let copy_size = (slice_width * 4) as wgpu::BufferAddress;

                encoder.copy_buffer_to_buffer(
                    &tile.buffer,
                    src_offset,
                    &output_buffer,
                    dst_offset,
                    copy_size,
                );
            }
        }
    }

    queue.submit(Some(encoder.finish()));

    CaResult {
        buffer: output_buffer,
        simulated_width,
        visible_width,
        height: output_height,
        padding_left: padding,
    }
}

pub fn run_ca(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rule: u8,
    start_generation: u32,      // Which generation to start from (viewport offset_y)
    iterations: u32,             // How many generations to compute
    visible_width: u32,
    horizontal_offset: i32,      // Horizontal cell offset (viewport offset_x)
    initial_state: Option<String>,
) -> CaResult {
    // Add padding for boundary simulation
    // Pattern can expand by (start_generation + iterations) cells in each direction
    // because we compute from generation 0 through start_generation + iterations
    let total_generations = start_generation + iterations;
    let padding = total_generations;
    let simulated_width = visible_width + 2 * padding;

    println!("Visible width: {}, Simulated width: {} (padding: {})", visible_width, simulated_width, padding);
    println!("Computing generations {} to {}, horizontal offset: {}",
        start_generation, start_generation + iterations, horizontal_offset);

    // We need to compute all generations from 0 to start_generation + iterations
    // (Phase 4b will add caching to avoid recomputing earlier generations)
    let total_iterations = start_generation + iterations;
    let buffer_height = total_iterations + 1;

    // Initialize first row (generation 0) with padding
    let mut initial_row = vec![0u32; simulated_width as usize];

    if let Some(state_str) = initial_state {
        // Parse user-provided initial state
        // World cell W maps to buffer index: padding + (W - horizontal_offset)
        // So initial state (centered at world 0) starts at: padding - horizontal_offset
        let base_offset = padding as i32 - horizontal_offset;
        for (i, ch) in state_str.chars().enumerate() {
            let pos = base_offset + i as i32;
            if pos >= 0 && (pos as usize) < simulated_width as usize {
                initial_row[pos as usize] = if ch == '1' { 1 } else { 0 };
            }
        }
    } else {
        // Default: single cell at world position 0
        // World cell 0 maps to buffer index: padding + (0 - horizontal_offset)
        let world_zero_in_buffer = padding as i32 - horizontal_offset;
        if world_zero_in_buffer >= 0 && (world_zero_in_buffer as usize) < simulated_width as usize {
            initial_row[world_zero_in_buffer as usize] = 1;
        }
    }

    // Create buffer for all iterations from gen 0 to start + visible
    let total_cells = simulated_width * buffer_height;

    // Initialize buffer with first row
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..simulated_width as usize].copy_from_slice(&initial_row);

    // Create single buffer (no ping-pong needed since we read from row N and write to row N+1)
    let ca_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    // Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("CA Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ca_compute.wgsl").into()),
    });

    // Create bind group layout (single buffer for both read and write, plus params)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("CA Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Create compute pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("CA Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("CA Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Create a single command encoder for ALL iterations
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("CA Compute Encoder"),
    });

    // Dispatch all iterations with ping-pong buffers
    let workgroups = (simulated_width + 255) / 256;

    for iter in 0..total_iterations {
        let params = Params {
            width: simulated_width,
            height: buffer_height,
            rule: rule as u32,
            current_row: iter,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Use single buffer (reads from current_row, writes to current_row + 1)
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CA Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ca_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CA Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    // Submit compute work
    queue.submit(Some(encoder.finish()));

    // Create output buffer containing only the visible range (start_generation to start_generation + iterations)
    let visible_height = iterations + 1;
    let visible_buffer_size = (simulated_width * visible_height * 4) as wgpu::BufferAddress;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Visible Range Buffer"),
        size: visible_buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Copy visible range from CA buffer
    let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });

    let source_offset = (start_generation * simulated_width * 4) as wgpu::BufferAddress;
    copy_encoder.copy_buffer_to_buffer(
        &ca_buffer,
        source_offset,
        &output_buffer,
        0,
        visible_buffer_size,
    );

    queue.submit(Some(copy_encoder.finish()));

    CaResult {
        buffer: output_buffer,
        simulated_width,
        visible_width,
        height: visible_height,
        padding_left: padding,
    }
}
