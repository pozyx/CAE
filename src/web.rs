// Web-specific entry point and initialization
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{render::RenderApp, Config};

// Flag to signal viewport reset from JavaScript
pub(crate) static RESET_VIEWPORT_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Request a viewport reset (called from JavaScript)
#[wasm_bindgen]
pub fn reset_viewport() {
    RESET_VIEWPORT_REQUESTED.store(true, Ordering::SeqCst);
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
        config.cell_size,
        config.cache_tiles,
        config.initial_state.clone(),
        config.zoom_min,
        config.zoom_max,
    ).await
}

/// Start the application with specific parameters
/// Called from JavaScript with values from the UI form
#[wasm_bindgen]
pub async fn start_with_params(
    rule: u8,
    width: u32,
    height: u32,
    cell_size: u32,
    cache_tiles: usize,
    initial_state: Option<String>,
    zoom_min: f32,
    zoom_max: f32,
) -> Result<(), JsValue> {
    // Set up panic hook and logger only once (ignore errors if already initialized)
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // Ignore error if logger is already initialized
        let _ = console_log::init_with_level(log::Level::Info);
    });

    log::info!("Starting CAE with rule {}, {}x{}, cell size {}", rule, width, height, cell_size);

    let config = Config {
        rule,
        initial_state,
        width,
        height,
        cell_size,
        zoom_min,
        zoom_max,
        debounce_ms: 100,
        fullscreen: false,
        cache_tiles,
    };

    let event_loop = EventLoop::new()
        .map_err(|e| JsValue::from_str(&format!("Failed to create event loop: {:?}", e)))?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let app = RenderApp::new(&event_loop, config).await;

    // On web, run_app doesn't return - it transfers control to the browser
    let _ = event_loop.run_app(&mut { app });

    Ok(())
}
