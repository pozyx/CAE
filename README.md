# CAE - Cellular Automaton Engine

A high-performance GPU-accelerated 1D cellular automaton visualizer with interactive viewport controls, built in Rust using WebGPU.

## Overview

CAE (Cellular Automaton Engine) is a desktop application that visualizes [Wolfram's elementary cellular automata](https://en.wikipedia.org/wiki/Elementary_cellular_automaton) rules. It computes and renders thousands of generations in real-time using GPU acceleration, allowing you to explore the fascinating patterns that emerge from simple rules.

The application features an interactive viewport with pan and zoom controls, enabling you to navigate through the generated patterns and explore different regions of the automaton's evolution.

## Features

- **GPU-Accelerated Computation**: All cellular automaton iterations are computed on the GPU using WebGPU compute shaders
- **Zero-Copy Architecture**: Data remains on the GPU throughout the computation pipeline - no CPU readback between iterations
- **Interactive Viewport**:
  - Pan by dragging with the mouse
  - Zoom in/out using the scroll wheel
  - Configurable zoom limits
- **Flexible Configuration**:
  - Support for all 256 Wolfram elementary CA rules (0-255)
  - Customizable initial states (binary strings or default single-cell)
  - Adjustable cell size and window dimensions
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
  -f, --fullscreen                     Start in fullscreen mode
  -h, --help                           Print help
```

### Interactive Controls

- **Drag mouse**: Pan the viewport
- **Scroll wheel**: Zoom in/out
- **F11**: Toggle fullscreen mode
- **ESC**: Exit application

## How It Works

### Architecture

CAE uses a GPU-accelerated pipeline with three main components:

1. **Compute Module** (`src/compute.rs`):
   - Implements Wolfram CA rules using WebGPU compute shaders
   - Computes all generations on the GPU in a single dispatch
   - Uses bit manipulation to evaluate the 3-cell neighborhood and apply rules

2. **Render Module** (`src/render.rs`):
   - Manages the application window and event loop
   - Handles user input (pan, zoom, resize)
   - Orchestrates viewport changes and triggers recomputation
   - Renders the visible portion of the CA state

3. **Shaders**:
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

- Phase 4b: Incremental computation cache with eviction strategy
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
