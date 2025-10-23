use wgpu::util::DeviceExt;

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
) -> CaResult {
    // Add padding for boundary simulation
    // Pattern can expand `iterations` cells in each direction from the visible area
    let padding = iterations;
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
        // Parse user-provided initial state (centered in simulated space, adjusted for horizontal offset)
        let base_offset = (simulated_width / 2) as i32 - horizontal_offset;
        for (i, ch) in state_str.chars().enumerate() {
            let pos = base_offset + i as i32;
            if pos >= 0 && (pos as usize) < simulated_width as usize {
                initial_row[pos as usize] = if ch == '1' { 1 } else { 0 };
            }
        }
    } else {
        // Default: single cell at world position 0 (center), adjusted for horizontal offset
        let world_center_in_sim = (simulated_width / 2) as i32 - horizontal_offset;
        if world_center_in_sim >= 0 && (world_center_in_sim as usize) < simulated_width as usize {
            initial_row[world_center_in_sim as usize] = 1;
        }
    }

    // Create buffer for all iterations from gen 0 to start + visible
    let total_cells = simulated_width * buffer_height;

    // Initialize both buffers with first row
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..simulated_width as usize].copy_from_slice(&initial_row);

    // Create ping-pong buffers (need STORAGE for compute and COPY_SRC for extracting visible range)
    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer A"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer B"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    // Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("CA Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ca_compute.wgsl").into()),
    });

    // Create bind group layout (now with 3 bindings: input, output, params)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("CA Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
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

        // Ping-pong: even iterations A->B, odd iterations B->A
        let (read_buffer, write_buffer) = if iter % 2 == 0 {
            (&buffer_a, &buffer_b)
        } else {
            (&buffer_b, &buffer_a)
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CA Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: read_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: write_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
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

    // Determine which buffer has the final result
    let source_buffer = if total_iterations % 2 == 0 { &buffer_a } else { &buffer_b };

    // Create output buffer containing only the visible range (start_generation to start_generation + iterations)
    let visible_height = iterations + 1;
    let visible_buffer_size = (simulated_width * visible_height * 4) as wgpu::BufferAddress;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Visible Range Buffer"),
        size: visible_buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Copy visible range from source buffer
    let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });

    let source_offset = (start_generation * simulated_width * 4) as wgpu::BufferAddress;
    copy_encoder.copy_buffer_to_buffer(
        source_buffer,
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
