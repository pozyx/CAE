use wgpu;

fn main() {
    env_logger::init();
    pollster::block_on(run());
}

async fn run() {
    // Initialize wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Request an adapter (physical GPU)
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Get adapter info
    let info = adapter.get_info();
    println!("Using GPU: {} ({:?})", info.name, info.backend);

    // Request a device and queue
    let (device, _queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    println!("WebGPU initialized successfully!");
    println!("Device: {:?}", device);
}
