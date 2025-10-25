# CAE - Web Version

This guide explains how to build and deploy the WebAssembly version of CAE that runs in web browsers.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition or later)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) for building WebAssembly

### Installing wasm-pack

```bash
cargo install wasm-pack --locked
```

## Building the Web Version

To build the WebAssembly package:

```bash
wasm-pack build --target web --features web
```

This will:
1. Compile the Rust code to WebAssembly
2. Generate JavaScript bindings
3. Create a `pkg/` directory with the WASM module and JS glue code
4. Optimize the WASM binary

The build output will be in the `pkg/` directory and includes:
- `caelib_bg.wasm` - The WebAssembly binary
- `caelib.js` - JavaScript bindings
- `caelib.d.ts` - TypeScript definitions
- `package.json` - NPM package metadata

## Running Locally

You need to serve the files over HTTP (not `file://`) because of WebAssembly security requirements.

### Option 1: Python HTTP Server

```bash
# Python 3
python -m http.server 8000

# Then open http://localhost:8000 in your browser
```

### Option 2: Node.js http-server

```bash
# Install globally
npm install -g http-server

# Run server
http-server -p 8000

# Then open http://localhost:8000 in your browser
```

### Option 3: Any static file server

Any HTTP server that can serve static files will work. Just make sure:
- The `src/index.html` is accessible at the root URL (use a symlink or configure your server)
- The `pkg/` directory is accessible
- MIME types are set correctly (most servers do this automatically)

## Deployment

### GitHub Pages

1. **Build the WebAssembly package:**
   ```bash
   wasm-pack build --target web --features web
   ```

2. **Commit the files:**
   ```bash
   git add src/index.html pkg/
   git commit -m "Add web version"
   git push
   ```

3. **Enable GitHub Pages:**
   - Go to your repository settings
   - Navigate to "Pages" section
   - Set source to "Deploy from a branch"
   - Select your branch (e.g., `master` or `main`)
   - Save

4. **Access your app:**
   - Your app will be available at `https://<username>.github.io/<repository>/`
   - For example: `https://pozyx.github.io/CAE/`

### Other Static Hosting Services

The web version can be deployed to any static hosting service:

- **Netlify**: Drag and drop the directory (with `src/index.html` and `pkg/`)
- **Vercel**: Deploy via CLI or GitHub integration
- **Cloudflare Pages**: Connect your repository and deploy
- **AWS S3**: Upload files and configure bucket for static website hosting

## Project Structure

```
CAE/
├── src/
│   ├── index.html     # Web UI and application entry point
│   ├── shaders/       # WGSL compute and render shaders
│   │   ├── ca_compute.wgsl
│   │   └── render.wgsl
│   ├── lib.rs         # Shared library code
│   ├── main.rs        # Desktop entry point
│   ├── web.rs         # Web entry point (wasm-bindgen exports)
│   ├── render.rs      # Rendering logic (works on both desktop and web)
│   ├── compute.rs     # GPU compute shaders
│   └── cache.rs       # Tile caching system
├── pkg/               # Generated WASM package (created by wasm-pack)
│   ├── caelib_bg.wasm # WebAssembly binary
│   ├── caelib.js      # JavaScript bindings
│   └── ...
└── Cargo.toml         # Dependencies with desktop/web features
```

## Features

The web version includes all core features of the desktop version:

- **GPU-Accelerated Computation**: WebGPU compute shaders
- **Interactive Viewport**: Pan with mouse, zoom with scroll wheel
- **Tile-Based Caching**: LRU cache for efficient navigation
- **Visual Cell Editor**: Click to toggle cells in the initial state
- **Configurable Parameters**:
  - Basic: Rule, initial state, cell size
  - Advanced: Cache tiles, zoom limits

## Browser Compatibility

The web version requires WebGPU support:

- **Chrome/Edge**: Version 113+ (enabled by default)
- **Firefox**: Version 121+ (experimental, enable in `about:config`)
- **Safari**: Version 18+ (Technical Preview)

Check browser compatibility at: https://caniuse.com/webgpu

## Controls

- **Drag mouse**: Pan the viewport
- **Scroll wheel**: Zoom in/out
- **0 key**: Reset viewport to initial position
- **UI Panel**: Adjust parameters and restart the simulation

## Development

### Desktop vs Web

The codebase supports both desktop and web builds using Cargo features:

```bash
# Build desktop version (default)
cargo build --release

# Build web version
wasm-pack build --target web --features web
```

### Code Organization

- **Shared code** (`src/lib.rs`, `src/render.rs`, `src/compute.rs`, `src/cache.rs`):
  - Works on both desktop and web
  - Uses `#[cfg(target_arch = "wasm32")]` for platform-specific code

- **Desktop-only** (`src/main.rs`):
  - CLI argument parsing with `clap`
  - Desktop window creation

- **Web-only** (`src/web.rs`):
  - WebAssembly exports with `wasm-bindgen`
  - Browser initialization

### Rebuilding After Changes

After modifying Rust code:

```bash
# Rebuild WASM
wasm-pack build --target web --features web

# Refresh your browser (the new WASM will be loaded)
```

After modifying `src/index.html`:
- Just refresh your browser (no rebuild needed)

## Troubleshooting

### "Failed to load WASM module"

- Make sure you're serving over HTTP (not `file://`)
- Check that the `pkg/` directory exists and contains `caelib_bg.wasm`
- Check browser console for detailed error messages

### WebGPU not available

- Verify your browser supports WebGPU
- Check that hardware acceleration is enabled in browser settings
- Try Chrome/Edge 113+ for best compatibility

### Build errors

- Ensure Rust and wasm-pack are up to date
- Make sure you're building with `--features web`
- Check that all dependencies in `Cargo.toml` are compatible

### Performance issues

- Try reducing cache size in the Advanced settings
- Use smaller cell sizes for better frame rates
- Check GPU usage in browser dev tools

## Future Enhancements

Potential additions to the web version:

- URL parameters for sharing specific configurations
- Save/load patterns to local storage
- Export images or animations
- Touch controls for mobile devices
- Progressive Web App (PWA) support

## License

MIT License - see LICENSE file for details
