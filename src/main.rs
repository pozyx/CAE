use clap::Parser;
use winit::event_loop::{ControlFlow, EventLoop};

mod compute;
mod render;

#[derive(Parser, Debug, Clone)]
#[command(name = "CAE")]
#[command(about = "1D Cellular Automaton Engine with GPU acceleration", long_about = None)]
pub struct Args {
    /// Wolfram CA rule number (0-255)
    #[arg(short, long)]
    pub rule: u8,

    /// Number of iterations to compute
    #[arg(short, long)]
    pub iterations: u32,

    /// Grid width (auto-expands if not specified)
    #[arg(short, long)]
    pub width: Option<u32>,

    /// Initial state as binary string (e.g., "00100" for center cell)
    #[arg(short = 's', long)]
    pub initial_state: Option<String>,

    /// Render mode: "none" (ASCII only), "static" (all at once), or "animated" (row-by-row)
    #[arg(short = 'm', long, default_value = "none")]
    pub render_mode: String,

    /// Window width for rendering (default: grid width)
    #[arg(long)]
    pub window_width: Option<u32>,

    /// Window height for rendering (default: iterations + 1)
    #[arg(long)]
    pub window_height: Option<u32>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    println!("1D Cellular Automaton Engine");
    println!("Rule: {}", args.rule);
    println!("Iterations: {}", args.iterations);
    println!("Render mode: {}", args.render_mode);

    match args.render_mode.as_str() {
        "none" => {
            // ASCII mode - no window needed
            pollster::block_on(run_ascii(args));
        }
        "static" | "animated" => {
            // Rendering mode - needs event loop
            run_render(args);
        }
        _ => {
            eprintln!("Invalid render mode. Use 'none', 'static', or 'animated'");
            std::process::exit(1);
        }
    }
}

async fn run_ascii(args: Args) {
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

fn run_render(args: Args) {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = pollster::block_on(render::RenderApp::new(&event_loop, args));

    event_loop.run_app(&mut { app }).expect("Failed to run event loop");
}
