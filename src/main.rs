use clap::Parser;
use winit::event_loop::{ControlFlow, EventLoop};

mod cache;
mod compute;
mod render;

#[derive(Parser, Debug, Clone)]
#[command(name = "CAE")]
#[command(about = "1D Cellular Automaton Engine with GPU acceleration", long_about = None)]
pub struct Args {
    /// Wolfram CA rule number (0-255)
    #[arg(short, long)]
    pub rule: u8,

    /// Initial state as binary string (e.g., "00100" for center cell)
    #[arg(short = 's', long)]
    pub initial_state: Option<String>,

    /// Window width in pixels (default: 800)
    #[arg(short, long, default_value = "800")]
    pub width: u32,

    /// Window height in pixels (default: 600)
    #[arg(long, default_value = "600")]
    pub height: u32,

    /// Cell size in pixels (default: 10, each cell is NxN pixels)
    #[arg(short = 'c', long, default_value = "10")]
    pub cell_size: u32,

    /// Minimum zoom level (default: 0.1)
    #[arg(long, default_value = "0.1")]
    pub zoom_min: f32,

    /// Maximum zoom level (default: 10.0)
    #[arg(long, default_value = "10.0")]
    pub zoom_max: f32,

    /// Debounce time in milliseconds before recomputing after viewport change (default: 100)
    #[arg(long, default_value = "100")]
    pub debounce_ms: u64,

    /// Start in fullscreen mode
    #[arg(short = 'f', long, default_value = "false")]
    pub fullscreen: bool,

    /// Maximum number of tiles to cache (0 to disable caching, default: 64)
    #[arg(long, default_value = "64")]
    pub cache_tiles: usize,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let initial_display = args.initial_state.as_ref()
        .map(|s| if s.len() > 30 { format!("{}...", &s[..27]) } else { s.clone() })
        .unwrap_or_else(|| "1 (single cell)".to_string());

    // Box width: 48 characters inside the borders
    // Format: "║ Label: value{padding}║"
    // "Rule: " = 6 chars, so padding = 48 - 6 - value_len
    // "Initial State: " = 15 chars, so padding = 48 - 15 - value_len

    println!("╔════════════════════════════════════════════════╗");
    println!("║   CAE - Cellular Automaton Engine              ║");
    println!("╠════════════════════════════════════════════════╣");
    println!("║ Rule: {:<40} ║", args.rule);
    println!("║ Initial State: {:<31} ║", initial_display);
    println!("╠════════════════════════════════════════════════╣");
    println!("║ Controls:                                      ║");
    println!("║  • Drag mouse: Pan viewport                    ║");
    println!("║  • Scroll wheel: Zoom in/out                   ║");
    println!("║  • 0: Reset viewport to initial position       ║");
    println!("║  • F11: Toggle fullscreen                      ║");
    println!("║  • ESC: Exit                                   ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = pollster::block_on(render::RenderApp::new(&event_loop, args));

    event_loop.run_app(&mut { app }).expect("Failed to run event loop");
}
