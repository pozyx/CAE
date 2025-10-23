use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    width: u32,
    rule: u32,
    current_row: u32,
    _padding: u32,
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

    // Create buffer for all iterations (width x (iterations + 1))
    let total_cells = width * (iterations + 1);
    let buffer_size = (total_cells * 4) as u64;

    // Initialize storage buffer with first row
    let mut all_data = vec![0u32; total_cells as usize];
    all_data[0..width as usize].copy_from_slice(&initial_row);

    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("CA State Buffer"),
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
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });

    // Run iterations
    for iter in 0..iterations {
        let params = Params {
            width,
            rule: rule as u32,
            current_row: iter,
            _padding: 0,
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
                    resource: storage_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("CA Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CA Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (workgroup_size is 256 in shader)
            let workgroups = (width + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        queue.submit(Some(encoder.finish()));
    }

    // Copy result to staging buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, buffer_size);
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
    for row in 0..=iterations {
        let start = (row * width) as usize;
        let end = start + width as usize;
        result.push(result_data[start..end].to_vec());
    }

    result
}
