use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    width: u32,
    height: u32,
    rule: u32,
    current_row: u32,
}

pub async fn run_ca(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rule: u8,
    iterations: u32,
    width_opt: Option<u32>,
    initial_state: Option<String>,
) -> Vec<Vec<u32>> {
    // Determine width
    let width = width_opt.unwrap_or_else(|| {
        // Auto-calculate width: start with a reasonable size
        // For center-cell initialization, we need at least iterations * 2 + 1
        // to avoid boundary issues
        (iterations * 2 + 1).max(256)
    });

    let height = iterations + 1;

    println!("Grid width: {}", width);

    // Initialize first row
    let mut initial_row = vec![0u32; width as usize];

    if let Some(state_str) = initial_state {
        // Parse user-provided initial state
        for (i, ch) in state_str.chars().enumerate() {
            if i >= width as usize {
                break;
            }
            initial_row[i] = if ch == '1' { 1 } else { 0 };
        }
    } else {
        // Default: single cell in center
        let center = width as usize / 2;
        initial_row[center] = 1;
    }

    // Create buffer for all iterations (width x height)
    let total_cells = width * height;
    let buffer_size = (total_cells * 4) as u64;

    // Initialize both buffers with first row
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..width as usize].copy_from_slice(&initial_row);

    // Create ping-pong buffers
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

    // Create staging buffer for reading results back
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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
    let workgroups = (width + 255) / 256;

    for iter in 0..iterations {
        let params = Params {
            width,
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

    // Copy result from the final buffer
    let final_buffer = if iterations % 2 == 0 { &buffer_a } else { &buffer_b };
    encoder.copy_buffer_to_buffer(final_buffer, 0, &staging_buffer, 0, buffer_size);

    // Submit ALL work at once
    queue.submit(Some(encoder.finish()));

    // Read back results
    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    // Convert flat buffer to 2D vector
    let mut result = Vec::new();
    for row in 0..height {
        let start = (row * width) as usize;
        let end = start + width as usize;
        result.push(result_data[start..end].to_vec());
    }

    result
}
