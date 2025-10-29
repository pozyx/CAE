// Shared library code for both desktop and web versions

pub mod cache;
pub mod compute;
pub mod render;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Configuration for the CA engine
/// This is a plain struct without CLI dependencies, usable from both desktop and web
#[derive(Debug, Clone)]
pub struct Config {
    /// Wolfram CA rule number (0-255)
    pub rule: u8,

    /// Initial state as binary string (e.g., "00100" for center cell)
    pub initial_state: Option<String>,

    /// Window width in pixels
    pub width: u32,

    /// Window height in pixels
    pub height: u32,

    /// Cell size in pixels (each cell is NxN pixels)
    pub cell_size: u32,

    /// Minimum zoom level
    pub zoom_min: f32,

    /// Maximum zoom level
    pub zoom_max: f32,

    /// Debounce time in milliseconds before recomputing after viewport change
    pub debounce_ms: u64,

    /// Start in fullscreen mode
    pub fullscreen: bool,

    /// Maximum number of tiles to cache (0 to disable caching)
    pub cache_tiles: usize,

    /// Tile size for caching (tiles are NxN cells, must be > 0)
    pub tile_size: u32,
}

impl Config {
    /// Validate and clamp tile_size to safe range
    pub fn validate_tile_size(&mut self) {
        if self.tile_size == 0 {
            eprintln!("Warning: tile_size cannot be 0, setting to default 256");
            self.tile_size = 256;
        }
        if self.tile_size < 64 {
            eprintln!("Warning: tile_size {} too small, clamping to 64", self.tile_size);
            self.tile_size = 64;
        }
        if self.tile_size > 1024 {
            eprintln!("Warning: tile_size {} too large, clamping to 1024", self.tile_size);
            self.tile_size = 1024;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rule: 30,
            initial_state: None,
            width: 1280,
            height: 960,
            cell_size: 10,
            zoom_min: 0.1,
            zoom_max: 10.0,
            debounce_ms: 100,
            fullscreen: false,
            cache_tiles: 64,
            tile_size: 256,
        }
    }
}
