// Shared library code for both desktop and web versions

pub mod cache;
pub mod compute;
pub mod render;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Global constants that can be tuned
pub mod constants {
    /// Default cell size in pixels (each cell is NxN pixels)
    pub const DEFAULT_CELL_SIZE: u32 = 10;

    /// Default window dimensions
    pub const DEFAULT_WIDTH: u32 = 1280;
    pub const DEFAULT_HEIGHT: u32 = 960;

    /// Default debounce time in milliseconds
    pub const DEFAULT_DEBOUNCE_MS: u64 = 0;

    /// Default cache settings
    /// 64 tiles @ 256x256 cells = ~64MB cache
    pub const DEFAULT_CACHE_TILES: usize = 64;

    pub const DEFAULT_TILE_SIZE: u32 = 256;

    /// Default rule
    pub const DEFAULT_RULE: u8 = 30;
}

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
    /// Validate all configuration parameters
    /// Returns Ok(()) if valid, or Err with a list of validation errors
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Rule is u8, always valid (0-255)

        // Initial state: must be empty or contain only 0s and 1s
        if let Some(ref state) = self.initial_state {
            if !state.is_empty() && !state.chars().all(|c| c == '0' || c == '1') {
                errors.push(format!("initial_state must be empty or contain only 0s and 1s"));
            }
        }

        // Window width/height validation: only for desktop
        // On web, canvas size is determined by the browser and can be any size (including mobile)
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Window width: 500-8192 pixels (GPU texture size limit)
            if self.width < 500 {
                errors.push(format!("width must be at least 500 (got {})", self.width));
            }
            if self.width > 8192 {
                errors.push(format!("width must be at most 8192 (got {})", self.width));
            }

            // Window height: 500-8192 pixels (GPU texture size limit)
            if self.height < 500 {
                errors.push(format!("height must be at least 500 (got {})", self.height));
            }
            if self.height > 8192 {
                errors.push(format!("height must be at most 8192 (got {})", self.height));
            }
        }

        // Cache tiles: 0-256
        if self.cache_tiles > 256 {
            errors.push(format!("cache_tiles must be at most 256 (got {})", self.cache_tiles));
        }

        // Tile size: 64-1024
        if self.tile_size < 64 {
            errors.push(format!("tile_size must be at least 64 (got {})", self.tile_size));
        }
        if self.tile_size > 1024 {
            errors.push(format!("tile_size must be at most 1024 (got {})", self.tile_size));
        }
        if self.tile_size == 0 {
            errors.push(format!("tile_size cannot be 0"));
        }

        // Debounce: 0-5000ms (0 = instant, 5s = very long delay)
        if self.debounce_ms > 5000 {
            errors.push(format!("debounce_ms must be at most 5000 (got {})", self.debounce_ms));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Legacy method for backward compatibility - now just calls validate
    #[deprecated(note = "Use validate() instead")]
    pub fn validate_tile_size(&mut self) {
        // Just validate tile_size specifically for backward compatibility
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
            rule: constants::DEFAULT_RULE,
            initial_state: None,
            width: constants::DEFAULT_WIDTH,
            height: constants::DEFAULT_HEIGHT,
            debounce_ms: constants::DEFAULT_DEBOUNCE_MS,
            fullscreen: false,
            cache_tiles: constants::DEFAULT_CACHE_TILES,
            tile_size: constants::DEFAULT_TILE_SIZE,
        }
    }
}
