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

    /// Initial state as binary string (e.g., "00100" for center cell)
    #[arg(short = 's', long)]
    pub initial_state: Option<String>,

    /// Window width (default: 800)
    #[arg(short, long, default_value = "800")]
    pub width: u32,

    /// Window height (default: 600)
    #[arg(long, default_value = "600")]
    pub height: u32,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    println!("1D Cellular Automaton Engine");
    println!("Rule: {}", args.rule);
    println!("Window: {}x{}", args.width, args.height);

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = pollster::block_on(render::RenderApp::new(&event_loop, args));

    event_loop.run_app(&mut { app }).expect("Failed to run event loop");
}
