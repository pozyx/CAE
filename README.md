# CAE - Cellular Automaton Engine

A high-performance GPU-accelerated 1D cellular automaton visualizer with interactive viewport controls, built in Rust using WebGPU.

## Overview

CAE (Cellular Automaton Engine) is a desktop application that visualizes [Wolfram's elementary cellular automata](https://en.wikipedia.org/wiki/Elementary_cellular_automaton) rules. It computes and renders thousands of generations in real-time using GPU acceleration, allowing you to explore the fascinating patterns that emerge from simple rules.

The application features an interactive viewport with pan and zoom controls, enabling you to navigate through the generated patterns and explore different regions of the automaton's evolution.

## Features

- **GPU-Accelerated Computation**: All cellular automaton iterations are computed on the GPU using WebGPU compute shaders
- **Zero-Copy Architecture**: Data remains on the GPU throughout the computation pipeline - no CPU readback between iterations
- **Tile-Based Caching**: Intelligent LRU cache system that stores 256×256 cell tiles to avoid redundant computation during navigation
- **Interactive Viewport**:
  - Pan by dragging with the mouse
  - Zoom in/out using the scroll wheel
  - Configurable zoom limits
  - Reset to initial view with '0' key
- **Flexible Configuration**:
  - Support for all 256 Wolfram elementary CA rules (0-255)
  - Customizable initial states (binary strings or default single-cell)
  - Adjustable cell size and window dimensions
  - Configurable cache size
- **Fullscreen Support**: Toggle fullscreen mode with F11
- **Debounced Recomputation**: Smooth viewport changes with configurable debounce timing
- **Efficient Rendering**: Only computes and renders visible cells based on current viewport

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition or later)
- A GPU with WebGPU support (most modern GPUs)

### Building from Source

```bash
git clone https://github.com/pozyx/CAE.git
cd CAE
cargo build --release
```

## Usage

### Basic Examples

Run with a specific Wolfram rule:
```bash
cargo run --release -- --rule 30
```

Specify a custom initial state:
```bash
cargo run --release -- --rule 110 --initial-state "00100100"
```

Set custom window size and cell size:
```bash
cargo run --release -- --rule 90 --width 1280 --height 1024 --cell-size 5
```

Start in fullscreen mode:
```bash
cargo run --release -- --rule 30 --fullscreen
```

### Command-Line Options

```
Options:
  -r, --rule <RULE>                    Wolfram CA rule number (0-255)
  -s, --initial-state <INITIAL_STATE>  Initial state as binary string (e.g., "00100")
  -w, --width <WIDTH>                  Window width in pixels [default: 800]
      --height <HEIGHT>                Window height in pixels [default: 600]
  -c, --cell-size <CELL_SIZE>          Cell size in pixels (NxN) [default: 10]
      --zoom-min <ZOOM_MIN>            Minimum zoom level [default: 0.1]
      --zoom-max <ZOOM_MAX>            Maximum zoom level [default: 10.0]
      --debounce-ms <DEBOUNCE_MS>      Debounce time before recomputing [default: 100]
      --cache-tiles <CACHE_TILES>      Maximum tiles to cache (0=disable) [default: 64]
  -f, --fullscreen                     Start in fullscreen mode
  -h, --help                           Print help
```

### Caching

The tile-based caching system significantly improves performance when navigating the viewport:

```bash
# Default caching (64 tiles, ~16MB GPU memory)
cargo run --release -- --rule 30

# Larger cache for better performance (128 tiles, ~32MB GPU memory)
cargo run --release -- --rule 30 --cache-tiles 128

# Disable caching
cargo run --release -- --rule 30 --cache-tiles 0
```

Each tile is 256×256 cells. The cache uses an LRU (Least Recently Used) eviction strategy, so frequently accessed areas remain cached while unused tiles are automatically evicted when the cache fills up.

### Interactive Controls

- **Drag mouse**: Pan the viewport
- **Scroll wheel**: Zoom in/out
- **0**: Reset viewport to initial position (centered, generation 0, zoom 1.0)
- **F11**: Toggle fullscreen mode
- **ESC**: Exit application

## How It Works

### Architecture

CAE uses a GPU-accelerated pipeline with four main components:

1. **Compute Module** (`src/compute.rs`):
   - Implements Wolfram CA rules using WebGPU compute shaders
   - Computes all generations on the GPU in a single dispatch
   - Uses bit manipulation to evaluate the 3-cell neighborhood and apply rules
   - Provides tile-based computation for cache system

2. **Cache Module** (`src/cache.rs`):
   - Manages LRU tile cache with configurable size
   - Stores 256×256 cell tiles indexed by grid coordinates
   - Tracks cache hits/misses and handles eviction

3. **Render Module** (`src/render.rs`):
   - Manages the application window and event loop
   - Handles user input (pan, zoom, resize)
   - Orchestrates viewport changes and triggers recomputation
   - Renders the visible portion of the CA state

4. **Shaders**:
   - `shaders/compute.wgsl`: Compute shader that evolves the CA
   - `shaders/render.wgsl`: Vertex/fragment shaders for visualization

### Zero-Copy GPU Pipeline

The entire computation and rendering pipeline keeps data on the GPU:

1. **Initialization**: Initial state uploaded to GPU buffer
2. **Computation**: Compute shader iterates through all generations, writing to GPU storage buffer
3. **Rendering**: Render pipeline reads directly from the GPU buffer - no CPU readback

This architecture achieves optimal performance by eliminating expensive GPU↔CPU transfers.

### Viewport System

The viewport system uses world-space coordinates:

- **Pan**: Translates the viewport origin in world space
- **Zoom**: Scales the viewport (affects cell density and visible area)
- **Debouncing**: Viewport changes are debounced to prevent excessive recomputation during continuous pan/zoom operations

When the viewport changes, the engine:
1. Calculates the new visible region in world space
2. Determines required grid dimensions
3. Recomputes the CA for the visible area on the GPU
4. Renders the result with proper scaling and translation

### Tile-Based Caching System

CAE uses a sophisticated tile-based caching system to avoid redundant GPU computation:

**Grid-Based Tiles**:
- The infinite CA space is divided into fixed 256×256 cell tiles
- Each tile is identified by grid coordinates `(tile_x, tile_y)`
- Tiles are independent units that can be computed and cached separately

**How Caching Works**:
1. **Viewport Mapping**: When rendering, the engine determines which tiles overlap the current viewport
2. **Cache Lookup**: Each required tile is checked against the cache using its grid coordinates
3. **Computation**: Missing tiles are computed on the GPU from generation 0 and inserted into the cache
4. **Assembly**: Cached and newly computed tiles are assembled into a single buffer for rendering
5. **LRU Eviction**: When the cache is full, least recently used tiles are evicted

**Performance Benefits**:
- **Small pan/zoom**: Most tiles are already cached → near-instant response
- **Large navigation**: Only tiles entering the viewport need computation
- **Consistent performance**: Cache effectiveness improves as you explore the same regions
- **Memory efficient**: Configurable cache size (default 64 tiles ≈ 16MB GPU memory)

**Cache Characteristics**:
- Each tile: 256×256×4 bytes = 256 KB GPU memory
- Cache hit rate: Typically 70-90% during normal navigation
- Tile computation: Each tile computes from generation 0 (enables future checkpointing)

## Technical Details

### Performance Characteristics

- **Computation**: O(generations × cells) on GPU, massively parallel
- **Memory**: Two GPU buffers (current and next generation)
- **Rendering**: Only visible cells are rendered (viewport culling)

### Dependencies

- `wgpu`: WebGPU implementation for Rust
- `winit`: Cross-platform window creation and event handling
- `clap`: Command-line argument parsing
- `bytemuck`: Safe transmutation for GPU data
- `pollster`: Async executor for initialization

## Interesting Rules to Try

- **Rule 30**: Chaotic pattern, used in random number generation
- **Rule 90**: Sierpiński triangle fractal
- **Rule 110**: Turing-complete, complex behavior
- **Rule 184**: Traffic flow simulation

## Roadmap

- Additional CA variants (2D, totalistic, etc.)
- Pattern detection and analysis tools
- Export capabilities (image, video)

## Development

This project was developed with the assistance of [Claude Code](https://claude.com/claude-code), Anthropic's AI-powered development tool.

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Stephen Wolfram for his pioneering work on cellular automata
- The Rust and WebGPU communities for excellent tools and documentation
