use clap::Parser;

mod compute;

#[derive(Parser, Debug)]
#[command(name = "CAE")]
#[command(about = "1D Cellular Automaton Engine with GPU acceleration", long_about = None)]
struct Args {
    /// Wolfram CA rule number (0-255)
    #[arg(short, long)]
    rule: u8,

    /// Number of iterations to compute
    #[arg(short, long)]
    iterations: u32,

    /// Grid width (auto-expands if not specified)
    #[arg(short, long)]
    width: Option<u32>,

    /// Initial state as binary string (e.g., "00100" for center cell)
    #[arg(short = 's', long)]
    initial_state: Option<String>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    println!("1D Cellular Automaton Engine");
    println!("Rule: {}", args.rule);
    println!("Iterations: {}", args.iterations);

    pollster::block_on(run(args));
}

async fn run(args: Args) {
    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let info = adapter.get_info();
    println!("Using GPU: {} ({:?})", info.name, info.backend);

    let (device, queue) = adapter
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

    // Run the cellular automaton
    let result = compute::run_ca(&device, &queue, args.rule, args.iterations, args.width, args.initial_state).await;

    // Display result as ASCII art
    compute::display_ascii(&result);
}
