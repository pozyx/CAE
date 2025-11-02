// Web-specific entry point and initialization
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{render::RenderApp, Config};

// Flag to signal viewport reset from JavaScript
pub(crate) static RESET_VIEWPORT_REQUESTED: AtomicBool = AtomicBool::new(false);

// Viewport state exposed to JavaScript (for URL updates)
pub(crate) static VIEWPORT_OFFSET_X: Mutex<f32> = Mutex::new(0.0);
pub(crate) static VIEWPORT_OFFSET_Y: Mutex<f32> = Mutex::new(0.0);
pub(crate) static VIEWPORT_CELL_SIZE: AtomicU32 = AtomicU32::new(10);

// Initial viewport state (set from URL parameters)
pub(crate) static INITIAL_VIEWPORT_SET: AtomicBool = AtomicBool::new(false);
pub(crate) static INITIAL_OFFSET_X: Mutex<f32> = Mutex::new(0.0);
pub(crate) static INITIAL_OFFSET_Y: Mutex<f32> = Mutex::new(0.0);
pub(crate) static INITIAL_CELL_SIZE: AtomicU32 = AtomicU32::new(10);

/// Request a viewport reset (called from JavaScript)
#[wasm_bindgen]
pub fn reset_viewport() {
    RESET_VIEWPORT_REQUESTED.store(true, Ordering::SeqCst);
}

/// Get current viewport offset X (called from JavaScript for URL updates)
#[wasm_bindgen]
pub fn get_viewport_x() -> f32 {
    *VIEWPORT_OFFSET_X.lock().unwrap()
}

/// Get current viewport offset Y (called from JavaScript for URL updates)
#[wasm_bindgen]
pub fn get_viewport_y() -> f32 {
    *VIEWPORT_OFFSET_Y.lock().unwrap()
}

/// Get current cell size (called from JavaScript for URL updates)
#[wasm_bindgen]
pub fn get_cell_size() -> u32 {
    VIEWPORT_CELL_SIZE.load(Ordering::SeqCst)
}

/// Set initial viewport state from URL parameters (called from JavaScript)
#[wasm_bindgen]
pub fn set_initial_viewport(offset_x: f32, offset_y: f32, cell_size: u32) {
    *INITIAL_OFFSET_X.lock().unwrap() = offset_x;
    *INITIAL_OFFSET_Y.lock().unwrap() = offset_y;
    INITIAL_CELL_SIZE.store(cell_size, Ordering::SeqCst);
    INITIAL_VIEWPORT_SET.store(true, Ordering::SeqCst);
}

/// Initialize the web application with default settings
/// This function is exported to JavaScript and can be called to start the app
#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    // Set up panic hook for better error messages in browser console
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Initialize console logging
    console_log::init_with_level(log::Level::Info)
        .expect("Failed to initialize logger");

    log::info!("CAE WebAssembly module loaded");

    // Create default configuration
    let config = Config::default();

    start_with_params(
        config.rule,
        config.width,
        config.height,
        config.initial_state.clone(),
    ).await
}

/// Start the application with specific parameters
/// Called from JavaScript with values from the UI form
#[wasm_bindgen]
pub async fn start_with_params(
    rule: u8,
    width: u32,
    height: u32,
    initial_state: Option<String>,
) -> Result<(), JsValue> {
    // Set up panic hook and logger only once (ignore errors if already initialized)
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // Ignore error if logger is already initialized
        let _ = console_log::init_with_level(log::Level::Info);
    });

    log::info!("Starting CAE with rule {}, {}x{}", rule, width, height);

    // Initialize viewport state globals to 0 to prevent stale values
    use std::sync::atomic::Ordering;
    *VIEWPORT_OFFSET_X.lock().unwrap() = 0.0;
    *VIEWPORT_OFFSET_Y.lock().unwrap() = 0.0;
    VIEWPORT_CELL_SIZE.store(10, Ordering::SeqCst);

    use crate::constants::{DEFAULT_CACHE_TILES, DEFAULT_TILE_SIZE};
    let config = Config {
        rule,
        initial_state,
        width,
        height,
        debounce_ms: 0,
        fullscreen: false,
        cache_tiles: DEFAULT_CACHE_TILES,
        tile_size: DEFAULT_TILE_SIZE,
    };

    // Validate configuration - this should never fail if JavaScript validation is correct,
    // but provides a safety layer in case JavaScript is bypassed
    if let Err(errors) = config.validate() {
        let error_msg = format!("Configuration validation failed:\n• {}", errors.join("\n• "));
        log::error!("{}", error_msg);
        return Err(JsValue::from_str(&error_msg));
    }

    let event_loop = EventLoop::new()
        .map_err(|e| JsValue::from_str(&format!("Failed to create event loop: {:?}", e)))?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let app = RenderApp::new(&event_loop, config).await;

    // On web, run_app doesn't return - it transfers control to the browser
    let _ = event_loop.run_app(&mut { app });

    Ok(())
}
