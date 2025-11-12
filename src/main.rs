use clap::Parser;
use winit::event_loop::{ControlFlow, EventLoop};

use caelib::Config;

#[derive(Parser, Debug, Clone)]
#[command(name = "CAE")]
#[command(about = "1D Cellular Automaton Engine with GPU acceleration", long_about = None)]
struct CliArgs {
    /// Wolfram CA rule number (0-255) [required]
    #[arg(short, long)]
    rule: u8,

    /// Initial state as binary string (e.g., "00100" for center cell) [default: single center cell]
    #[arg(long)]
    initial_state: Option<String>,

    /// Window width in pixels
    #[arg(long, default_value = "1280")]
    width: u32,

    /// Window height in pixels
    #[arg(long, default_value = "960")]
    height: u32,

    /// Start in fullscreen mode
    #[arg(short = 'f', long, default_value = "false")]
    fullscreen: bool,

    /// Debounce time in milliseconds before recomputing after viewport change
    #[arg(long, default_value = "0")]
    debounce_ms: u64,

    /// Maximum number of tiles to cache (0 to disable caching)
    #[arg(long, default_value = "64")]
    cache_tiles: usize,

    /// Cache tile size (tiles are NxN cells)
    #[arg(long, default_value = "256")]
    cache_tile_size: u32,
}

impl From<CliArgs> for Config {
    fn from(cli: CliArgs) -> Self {
        Config {
            rule: cli.rule,
            initial_state: cli.initial_state,
            width: cli.width,
            height: cli.height,
            debounce_ms: cli.debounce_ms,
            fullscreen: cli.fullscreen,
            cache_tiles: cli.cache_tiles,
            tile_size: cli.cache_tile_size,
        }
    }
}

fn main() {
    env_logger::init();

    // Parse CLI arguments with custom error handling
    let cli_args = match CliArgs::try_parse() {
        Ok(args) => args,
        Err(err) => {
            // Check if this is a help or version request (should exit with success)
            use clap::error::ErrorKind;
            match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    // Print help/version to stdout and exit successfully
                    print!("{}", err);
                    std::process::exit(0);
                }
                _ => {
                    // Actual error - format it to match our validation error format
                    let err_str = err.to_string();
                    let err_str = err_str.trim_end();

                    // Remove clap's "For more information" line if present
                    let err_lines: Vec<&str> = err_str.lines().collect();
                    let main_error = if err_lines.len() > 1 && err_lines.last().unwrap().contains("For more information") {
                        err_lines[..err_lines.len() - 2].join("\n") // Remove blank line and help line
                    } else {
                        err_str.to_string()
                    };

                    eprintln!("Error: {}", main_error);
                    eprintln!();
                    eprintln!("For more information, try '--help'.");
                    std::process::exit(1);
                }
            }
        }
    };

    let config: Config = cli_args.into();

    // Validate configuration before running
    if let Err(errors) = config.validate() {
        for error in &errors {
            eprintln!("Error: {}", error);
        }
        eprintln!();
        eprintln!("For more information, try '--help'.");
        std::process::exit(1);
    }

    let initial_display = config.initial_state.as_ref()
        .map(|s| if s.len() > 30 { format!("{}...", &s[..27]) } else { s.clone() })
        .unwrap_or_else(|| "1 (single cell)".to_string());

    // Box width: 48 characters inside the borders
    // Format: "║ Label: value{padding}║"
    // "Rule: " = 6 chars, so padding = 48 - 6 - value_len
    // "Initial State: " = 15 chars, so padding = 48 - 15 - value_len

    println!("╔══════════════════════════════════════════════════╗");
    println!("║   CAE - Cellular Automaton Engine                ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Rule: {:<40}   ║", config.rule);
    println!("║ Initial State: {:<31}   ║", initial_display);
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Controls:                                        ║");
    println!("║  • Drag to pan (mouse or touch)                  ║");
    println!("║  • Scroll wheel or pinch to zoom                 ║");
    println!("║  • 0: Reset viewport to initial position         ║");
    println!("║  • F11: Toggle fullscreen                        ║");
    println!("║  • ESC: Exit                                     ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    // Use Wait mode for on-demand rendering (only render when something changes)
    // This provides better battery life while maintaining full responsiveness
    event_loop.set_control_flow(ControlFlow::Wait);

    let app = pollster::block_on(caelib::render::RenderApp::new(&event_loop, config));

    event_loop.run_app(&mut { app }).expect("Failed to run event loop");
}
