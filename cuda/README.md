# CAE-CUDA - C++/CUDA Native Port

A C++17/CUDA port of [CAE](../README.md), optimized for NVIDIA GPUs. Uses CUDA for compute and OpenGL 4.5 for rendering, connected via CUDA-OpenGL interop for zero-copy buffer sharing.

## Requirements

- **CUDA Toolkit** 12.0+ (tested with 13.1)
- **CMake** 3.18+
- **C++17 compiler** (MSVC 2019+, GCC 9+, Clang 10+)
- **NVIDIA GPU** with compute capability 7.5+ (Turing or newer) and OpenGL 4.5 support

## Building

### Windows (Visual Studio + CUDA)

A PowerShell build script is provided:

```powershell
cd cuda
powershell -ExecutionPolicy Bypass -File build.ps1
```

The script sets up the VS developer environment, configures CMake with Ninja, and builds in Release mode. The executable is output to `build/CAE-CUDA.exe`.

### Generic CMake

```bash
cd cuda
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
```

You may need to specify the CUDA compiler path:
```bash
cmake -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_CUDA_COMPILER=/usr/local/cuda/bin/nvcc
```

## Usage

```bash
# Basic usage
./build/CAE-CUDA --rule 30

# Custom initial state
./build/CAE-CUDA --rule 110 --initial-state "00100100"

# Custom window size
./build/CAE-CUDA --rule 90 --width 1920 --height 1080

# Start in fullscreen
./build/CAE-CUDA --rule 30 --fullscreen

# Adjust cache (larger cache = better pan/zoom performance)
./build/CAE-CUDA --rule 30 --cache-tiles 128
```

### Command-Line Options

```
Options:
  -r, --rule UINT:INT in [0 - 255]     Wolfram CA rule number [default: 30]
      --initial-state TEXT              Initial state as binary string (e.g., "00100")
      --width UINT                      Window width in pixels [default: 1280]
      --height UINT                     Window height in pixels [default: 960]
      --debounce-ms UINT                Debounce time in ms before recomputing [default: 0]
      --cache-tiles UINT                Max tiles to cache, 0=disable [default: 64]
      --cache-tile-size UINT            Cache tile size in cells (NxN) [default: 256]
  -f, --fullscreen                      Start in fullscreen mode
  -h, --help                            Print help
```

### Controls

- **Drag** to pan (mouse or touch)
- **Scroll wheel** or **pinch** to zoom
- **0**: Reset viewport
- **F11**: Toggle fullscreen
- **ESC**: Exit fullscreen or quit

## Architecture

```
compute_kernel.cu  ──CUDA──►  SSBO (GPU)  ──OpenGL──►  Fragment Shader  ──►  Screen
                              ▲                                │
                              └── CUDA-GL interop (zero-copy) ─┘
```

- **CUDA compute kernel** (`compute_kernel.cu`): Applies Wolfram CA rules in parallel, one thread per cell
- **CUDA-OpenGL interop**: The CA state buffer is an OpenGL SSBO mapped into CUDA address space — no GPU-to-CPU-to-GPU copies
- **Fragment shader** (`shaders/render.frag`): Reads directly from the SSBO to render cells as pixels
- **Tile cache** (`cache.h/cpp`): LRU cache of 256x256 cell tiles to avoid recomputing previously visited regions

### Source Files

| File | Purpose |
|------|---------|
| `src/main.cpp` | CLI parsing (CLI11), entry point |
| `src/config.h/cpp` | Configuration, constants, validation |
| `src/compute.h/cpp` | Buffer management, CA orchestration, tile assembly |
| `src/compute_kernel.cu` | CUDA kernel for CA rule evaluation |
| `src/render.h/cpp` | Window, events, OpenGL rendering, touch input |
| `src/cache.h/cpp` | LRU tile cache |
| `src/viewport.h` | Viewport, drag state, touch state structs |
| `shaders/render.vert` | Fullscreen quad vertex shader |
| `shaders/render.frag` | CA state visualization fragment shader |

## Platform Notes

- **Windows**: Fully supported, including Win32 touch input, per-monitor DPI handling, and borderless fullscreen
- **Linux**: Builds and runs; touch input and DPI features degrade gracefully (not implemented, guarded by `#ifdef _WIN32`)
- **Hybrid GPU systems** (NVIDIA Optimus): The executable exports `NvOptimusEnablement = 1` to force the NVIDIA GPU for both CUDA and OpenGL, avoiding interop failures

## Differences from Rust/WebGPU Version

| Aspect | Rust/WebGPU | C++/CUDA |
|--------|------------|----------|
| GPU API | WebGPU (wgpu) | CUDA + OpenGL 4.5 |
| Windowing | winit | GLFW 3.4 |
| Shader language | WGSL | CUDA (compute) + GLSL (render) |
| Web support | Yes (WASM) | No |
| Touch input | Via winit | Win32 WM_TOUCH API |
| GPU selection | Automatic (wgpu) | NvOptimusEnablement export |
| Buffer sharing | wgpu managed | CUDA-GL interop (SSBO) |

This project was developed with the assistance of [Claude Code](https://claude.com/claude-code), Anthropic's AI-powered development tool.
