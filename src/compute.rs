use wgpu::util::DeviceExt;
use crate::cache::{TileCache, TileKey, Tile};

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

pub fn run_ca(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rule: u8,
    start_generation: u32,      // Which generation to start from (viewport offset_y)
    iterations: u32,             // How many generations to compute
    visible_width: u32,
    horizontal_offset: i32,      // Horizontal cell offset (viewport offset_x)
    initial_state: Option<String>,
    mut cache: Option<&mut TileCache>,  // Optional tile cache (mutable)
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

    // Check cache for this exact tile
    let horizontal_start = horizontal_offset - padding as i32;
    let horizontal_end = horizontal_offset + visible_width as i32 + padding as i32;
    let generation_end = start_generation + iterations;

    if let Some(ref mut cache_ref) = cache {
        let cache_key = TileKey::new(
            rule,
            &initial_state,
            start_generation,
            generation_end,
            horizontal_start,
            horizontal_end,
        );

        if let Some(cached_tile) = cache_ref.get(&cache_key) {
            println!("Using cached tile!");
            // Return cached result by creating a copy
            // Note: We need to clone the buffer, which requires a copy operation
            let buffer_size = (cached_tile.simulated_width * (generation_end - start_generation) * 4) as wgpu::BufferAddress;
            let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Cached Tile Output"),
                size: buffer_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cache Copy Encoder"),
            });

            encoder.copy_buffer_to_buffer(
                &cached_tile.buffer,
                0,
                &output_buffer,
                0,
                buffer_size,
            );

            queue.submit(Some(encoder.finish()));

            return CaResult {
                buffer: output_buffer,
                simulated_width: cached_tile.simulated_width,
                visible_width,
                height: generation_end - start_generation,
                padding_left: cached_tile.padding_left,
            };
        }
    }

    // We need to compute all generations from 0 to start_generation + iterations
    // (Phase 4b will add caching to avoid recomputing earlier generations)
    let total_iterations = start_generation + iterations;
    let buffer_height = total_iterations + 1;

    // Initialize first row (generation 0) with padding
    let mut initial_row = vec![0u32; simulated_width as usize];

    if let Some(ref state_str) = initial_state {
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
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ca_compute.wgsl").into()),
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
        entry_point: "main",
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
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
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

    // Insert into cache if caching is enabled
    // We need to create a separate buffer for the cache since buffers can't be cloned
    if let Some(ref mut cache_ref) = cache {
        let cache_key = TileKey::new(
            rule,
            &initial_state,
            start_generation,
            generation_end,
            horizontal_start,
            horizontal_end,
        );

        // Create a dedicated cache buffer
        let cache_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cached Tile Buffer"),
            size: visible_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Copy the output buffer to the cache buffer
        let mut cache_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Cache Insert Encoder"),
        });

        cache_encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &cache_buffer,
            0,
            visible_buffer_size,
        );

        queue.submit(Some(cache_encoder.finish()));

        let tile = Tile {
            buffer: cache_buffer,
            generation_start: start_generation,
            generation_end,
            horizontal_start,
            horizontal_end,
            simulated_width,
            padding_left: padding,
        };

        cache_ref.insert(cache_key, tile);
    }

    CaResult {
        buffer: output_buffer,
        simulated_width,
        visible_width,
        height: visible_height,
        padding_left: padding,
    }
}
