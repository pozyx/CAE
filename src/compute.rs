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
    iterations: u32,
    visible_width: u32,
    initial_state: Option<String>,
) -> CaResult {
    // Add padding for boundary simulation
    // Pattern can expand `iterations` cells in each direction
    let padding = iterations;
    let simulated_width = visible_width + 2 * padding;
    let height = iterations + 1;

    println!("Visible width: {}, Simulated width: {} (padding: {})", visible_width, simulated_width, padding);

    // Initialize first row with padding
    let mut initial_row = vec![0u32; simulated_width as usize];

    if let Some(state_str) = initial_state {
        // Parse user-provided initial state (centered in simulated space)
        let start_offset = padding as usize;
        for (i, ch) in state_str.chars().enumerate() {
            if i >= visible_width as usize {
                break;
            }
            initial_row[start_offset + i] = if ch == '1' { 1 } else { 0 };
        }
    } else {
        // Default: single cell in center of simulated space
        let center = simulated_width as usize / 2;
        initial_row[center] = 1;
    }

    // Create buffer for all iterations (simulated_width x height)
    let total_cells = simulated_width * height;

    // Initialize both buffers with first row
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..simulated_width as usize].copy_from_slice(&initial_row);

    // Create ping-pong buffers (need STORAGE usage for rendering too)
    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer A"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer B"),
        contents: bytemuck::cast_slice(&all_data),
        usage: wgpu::BufferUsages::STORAGE,
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

    for iter in 0..iterations {
        let params = Params {
            width: simulated_width,
            height,
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

    // Submit ALL compute work
    queue.submit(Some(encoder.finish()));

    // Return the final buffer (still on GPU!) with metadata
    let final_buffer = if iterations % 2 == 0 { buffer_a } else { buffer_b };

    CaResult {
        buffer: final_buffer,
        simulated_width,
        visible_width,
        height,
        padding_left: padding,
    }
}
