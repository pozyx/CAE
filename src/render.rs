use std::sync::Arc;

// Use web-time for cross-platform time support (works on both desktop and web)
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{cache::TileCache, compute, Config};

/// Viewport state in world space coordinates
#[derive(Debug, Clone)]
struct Viewport {
    /// Horizontal offset in cells (can be negative, 0 = initial cell centered)
    offset_x: f32,
    /// Vertical offset in cells (0 = generation 0, positive = later generations)
    offset_y: f32,
    /// Zoom level (1.0 = default, >1.0 = zoomed in, <1.0 = zoomed out)
    zoom: f32,
}

impl Viewport {
    fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }
}

/// Mouse drag state
#[derive(Debug, Clone)]
struct DragState {
    active: bool,
    start_x: f64,
    start_y: f64,
    viewport_at_start: Viewport,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] }, // Bottom-left
    Vertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },  // Bottom-right
    Vertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },   // Top-right
    Vertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },  // Top-left
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0,
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RenderParams {
    visible_width: u32,
    visible_height: u32,
    simulated_width: u32,
    padding_left: u32,
    cell_size: u32,
    window_width: u32,
    window_height: u32,
    viewport_offset_x: i32,  // Viewport offset for current view
    viewport_offset_y: i32,  // Viewport offset for current view
    buffer_offset_x: i32,    // Offset the buffer was computed for
    buffer_offset_y: i32,    // Offset the buffer was computed for
    _padding: u32,
}

pub struct RenderApp {
    config: Config,
    window: Option<Arc<Window>>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    ca_buffer: Option<wgpu::Buffer>,
    params_buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    bind_group_layout: wgpu::BindGroupLayout,

    // Viewport state
    viewport: Viewport,
    buffer_viewport: Viewport,  // Viewport that current CA buffer was computed for
    drag_state: Option<DragState>,
    last_viewport_change: Option<Instant>,
    needs_recompute: bool,
    cursor_position: (f64, f64),

    // Window and cell dimensions
    window_width: u32,
    window_height: u32,
    current_cell_size: u32,  // Runtime cell size (can be changed by zoom)

    // Tile cache
    cache: Option<TileCache>,

    // Track window position to detect which edge is being resized
    window_position: Option<(i32, i32)>,

    // Buffer metadata (from last CA computation)
    buffer_simulated_width: u32,
    buffer_padding_left: u32,

    // Stability: throttle render params updates
    last_params_update: Option<Instant>,
}

impl RenderApp {
    pub async fn new(_event_loop: &EventLoop<()>, config: Config) -> Self {
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let info = adapter.get_info();
        println!("Using GPU: {} ({:?})", info.name, info.backend);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Initial window dimensions
        let window_width = config.width;
        let window_height = config.height;

        println!("Initial window size: {}x{} pixels, cell size: {}px",
            window_width, window_height, config.cell_size);

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // Use Bgra8Unorm for web compatibility (some browsers don't support sRGB)
                    #[cfg(target_arch = "wasm32")]
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    #[cfg(not(target_arch = "wasm32"))]
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create params buffer (will be updated after CA computation)
        let params = RenderParams {
            visible_width: window_width / config.cell_size,
            visible_height: window_height / config.cell_size,
            simulated_width: window_width / config.cell_size,
            padding_left: 0,
            cell_size: config.cell_size,
            window_width,
            window_height,
            viewport_offset_x: 0,
            viewport_offset_y: 0,
            buffer_offset_x: 0,
            buffer_offset_y: 0,
            _padding: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let cell_size = config.cell_size;

        let cache_tiles = config.cache_tiles;

        Self {
            config,
            window: None,
            instance,
            adapter,
            surface: None,
            device,
            queue,
            surface_config: None,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            ca_buffer: None,
            params_buffer,
            bind_group: None,
            bind_group_layout,

            viewport: {
                let visible_cells_x = window_width as f32 / cell_size as f32;
                let mut vp = Viewport::new();
                // Center the origin (world position 0) horizontally
                vp.offset_x = -visible_cells_x / 2.0;
                vp
            },
            buffer_viewport: Viewport::new(),
            drag_state: None,
            last_viewport_change: None,
            needs_recompute: true,
            cursor_position: (window_width as f64 / 2.0, window_height as f64 / 2.0),

            window_width,
            window_height,
            current_cell_size: cell_size,

            cache: if cache_tiles > 0 {
                Some(TileCache::new(cache_tiles))
            } else {
                None
            },

            window_position: None,

            buffer_simulated_width: window_width / cell_size,
            buffer_padding_left: 0,

            last_params_update: None,
        }
    }

    fn init_window(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_width = self.window_width;
        let window_height = self.window_height;

        let mut window_attributes = Window::default_attributes()
            .with_title(format!("CAE - Cellular Automaton Engine | Rule {}", self.config.rule));

        // On desktop, set the window size. On web, don't - let it use the canvas's existing size
        #[cfg(not(target_arch = "wasm32"))]
        {
            window_attributes = window_attributes.with_inner_size(winit::dpi::PhysicalSize::new(window_width, window_height));
        }

        // Platform-specific setup
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            // Get the canvas element from the HTML document
            let web_window = web_sys::window().expect("Failed to get web window");
            let document = web_window.document().expect("Failed to get document");
            let canvas = document
                .get_element_by_id("cae-canvas")
                .expect("Failed to find canvas element with id 'cae-canvas'");
            let canvas: web_sys::HtmlCanvasElement = canvas
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("Element is not a canvas");

            window_attributes = window_attributes.with_canvas(Some(canvas));
        }

        // Set fullscreen if requested (desktop only)
        #[cfg(not(target_arch = "wasm32"))]
        if self.config.fullscreen {
            window_attributes = window_attributes.with_fullscreen(Some(
                winit::window::Fullscreen::Borderless(None)
            ));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Update actual window dimensions (may differ from requested if fullscreen)
        let actual_size = window.inner_size();

        // On web, if window reports size 0, fall back to config dimensions
        if actual_size.width > 0 && actual_size.height > 0 {
            self.window_width = actual_size.width;
            self.window_height = actual_size.height;
        } else {
            println!("Warning: Window reported size {}x{}, using config dimensions {}x{}",
                actual_size.width, actual_size.height, self.window_width, self.window_height);
        }

        // Create surface
        let surface = self.instance.create_surface(window.clone()).unwrap();

        let surface_caps = surface.get_capabilities(&self.adapter);

        // On web, prefer Bgra8Unorm for compatibility. On desktop, prefer sRGB.
        #[cfg(target_arch = "wasm32")]
        let surface_format = surface_caps.formats.iter()
            .find(|f| **f == wgpu::TextureFormat::Bgra8Unorm)
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        #[cfg(not(target_arch = "wasm32"))]
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.window_width,
            height: self.window_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&self.device, &config);

        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(config);

        // Now compute the CA
        self.compute_ca();
    }

    fn compute_ca(&mut self) {
        println!("Computing cellular automaton...");

        // Calculate visible cells based on window size, cell size, and zoom
        // Use ceil to include partial cells at the edges
        let visible_cells_x = ((self.window_width as f32 / self.current_cell_size as f32) / self.viewport.zoom).ceil() as u32;
        let visible_cells_y = ((self.window_height as f32 / self.current_cell_size as f32) / self.viewport.zoom).ceil() as u32;

        // Safety: limit maximum buffer dimensions to prevent GPU issues
        // More conservative limits to prevent driver crashes
        const MAX_CELLS_X: u32 = 5000;
        const MAX_CELLS_Y: u32 = 5000;
        const MIN_CELL_SIZE: u32 = 2;  // Prevent extremely small cells

        if self.current_cell_size < MIN_CELL_SIZE {
            eprintln!("Warning: Cell size {} is too small (minimum {})",
                self.current_cell_size, MIN_CELL_SIZE);
            eprintln!("Skipping computation to prevent GPU instability.");
            return;
        }

        if visible_cells_x > MAX_CELLS_X || visible_cells_y > MAX_CELLS_Y {
            eprintln!("Warning: Requested dimensions {}x{} exceed safety limits ({}x{})",
                visible_cells_x, visible_cells_y, MAX_CELLS_X, MAX_CELLS_Y);
            eprintln!("Skipping computation to prevent GPU instability.");
            return;
        }

        // Also check total cell count (width * height * padding factor)
        let total_cells = (visible_cells_x as u64 * 3) * visible_cells_y as u64;  // 3x for padding
        const MAX_TOTAL_CELLS: u64 = 10_000_000;  // 10 million cells max

        if total_cells > MAX_TOTAL_CELLS {
            eprintln!("Warning: Total cell count {} exceeds limit {}",
                total_cells, MAX_TOTAL_CELLS);
            eprintln!("Skipping computation to prevent GPU instability.");
            return;
        }

        // Clamp offset_y to not go below generation 0
        let clamped_offset_y = self.viewport.offset_y.max(0.0);

        // Start generation is the floored offset_y
        let start_generation = clamped_offset_y as u32;

        // Calculate number of iterations needed (visible generations)
        let iterations = visible_cells_y;

        // Horizontal offset in cells
        let horizontal_offset = self.viewport.offset_x as i32;

        println!("Viewport - offset: ({:.1}, {:.1}), zoom: {:.2}",
            self.viewport.offset_x, clamped_offset_y, self.viewport.zoom);
        println!("Visible cells: {}x{}, iterations: {}", visible_cells_x, visible_cells_y, iterations);

        // Run CA computation - result stays on GPU!
        let ca_result = if let Some(ref mut cache) = self.cache {
            // Use tile-based caching
            compute::run_ca_with_cache(
                &self.device,
                &self.queue,
                self.config.rule,
                start_generation,
                iterations,
                visible_cells_x,
                horizontal_offset,
                self.config.initial_state.clone(),
                cache,
            )
        } else {
            // No caching - use direct computation
            compute::run_ca(
                &self.device,
                &self.queue,
                self.config.rule,
                start_generation,
                iterations,
                visible_cells_x,
                horizontal_offset,
                self.config.initial_state.clone(),
            )
        };

        println!("CA result - Simulated: {}x{}, Visible: {}x{}, Padding: {}",
            ca_result.simulated_width, ca_result.height,
            ca_result.visible_width, ca_result.height,
            ca_result.padding_left);

        // Update render params with simulated grid info
        let params = RenderParams {
            visible_width: ca_result.visible_width,
            visible_height: ca_result.height,
            simulated_width: ca_result.simulated_width,
            padding_left: ca_result.padding_left,
            cell_size: self.current_cell_size,
            window_width: self.window_width,
            window_height: self.window_height,
            viewport_offset_x: self.viewport.offset_x as i32,
            viewport_offset_y: self.viewport.offset_y as i32,
            buffer_offset_x: self.viewport.offset_x as i32,  // Buffer just computed for current viewport
            buffer_offset_y: self.viewport.offset_y as i32,
            _padding: 0,
        };

        // Store the viewport this buffer was computed for
        self.buffer_viewport = self.viewport.clone();

        // Store buffer metadata for use in update_render_params()
        self.buffer_simulated_width = ca_result.simulated_width;
        self.buffer_padding_left = ca_result.padding_left;

        self.queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&[params]),
        );

        // Create bind group using GPU buffer directly (zero-copy!)
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ca_result.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        });

        self.ca_buffer = Some(ca_result.buffer);
        self.bind_group = Some(bind_group);
        self.needs_recompute = false;

        println!("Computation complete! (zero-copy GPU rendering)");
    }

    fn mark_viewport_changed(&mut self) {
        self.last_viewport_change = Some(Instant::now());
        self.needs_recompute = true;
    }

    fn update_render_params(&mut self) {
        // Throttle params updates to ~60 FPS to reduce GPU load
        // This prevents excessive buffer writes during rapid viewport changes
        if let Some(last_update) = self.last_params_update {
            if last_update.elapsed() < Duration::from_millis(16) {
                return;  // Skip update if less than 16ms since last update
            }
        }

        // Update render params to reflect current viewport vs buffer viewport
        // This allows immediate visual feedback during dragging/resizing
        let params = RenderParams {
            visible_width: ((self.window_width + self.current_cell_size - 1) / self.current_cell_size),  // Ceiling division
            visible_height: ((self.window_height + self.current_cell_size - 1) / self.current_cell_size),
            simulated_width: self.buffer_simulated_width,
            padding_left: self.buffer_padding_left,
            cell_size: self.current_cell_size,
            window_width: self.window_width,
            window_height: self.window_height,
            viewport_offset_x: self.viewport.offset_x as i32,
            viewport_offset_y: self.viewport.offset_y as i32,
            buffer_offset_x: self.buffer_viewport.offset_x as i32,
            buffer_offset_y: self.buffer_viewport.offset_y as i32,
            _padding: 0,
        };

        self.queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&[params]),
        );

        self.last_params_update = Some(Instant::now());
    }

    fn check_debounce_and_recompute(&mut self) {
        if let Some(last_change) = self.last_viewport_change {
            let debounce_duration = Duration::from_millis(self.config.debounce_ms);
            if last_change.elapsed() >= debounce_duration && self.needs_recompute {
                self.compute_ca();
                self.last_viewport_change = None;
            }
        }
    }

    pub fn reset_viewport(&mut self) {
        // Reset viewport to initial centered state
        println!("Resetting viewport to initial state...");
        self.current_cell_size = self.config.cell_size;
        self.viewport.zoom = 1.0;
        let visible_cells_x = self.window_width as f32 / self.current_cell_size as f32;
        self.viewport.offset_x = -visible_cells_x / 2.0;
        self.viewport.offset_y = 0.0;
        self.needs_recompute = true;
        self.last_viewport_change = Some(Instant::now());
    }

    fn handle_zoom(&mut self, delta: f32, cursor_x: f64, cursor_y: f64) {
        // Calculate zoom limits based on config
        // Zoom > 1.0 means zoomed IN (cells appear bigger)
        // Zoom < 1.0 means zoomed OUT (cells appear smaller)
        // zoom_factor = current_cell_size / base_cell_size
        let base_cell_size = self.config.cell_size;
        let min_cell_size = (base_cell_size as f32 * self.config.zoom_min).max(1.0) as u32;
        let max_cell_size = (base_cell_size as f32 * self.config.zoom_max) as u32;

        // Generate zoom levels dynamically based on limits
        let zoom_levels: Vec<u32> = {
            let mut levels = vec![
                2, 3, 4, 5, 6, 7, 8, 9, 10,
                12, 14, 16, 18, 20, 24, 28, 32, 36, 40,
                45, 50, 60, 70, 80, 90, 100, 120, 140, 160, 180, 200,
                250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000
            ];
            // Filter to only include levels within our zoom range
            levels.retain(|&size| size >= min_cell_size && size <= max_cell_size);
            levels
        };

        let old_cell_size = self.current_cell_size;

        // Find current zoom level index
        let current_index = zoom_levels.iter()
            .position(|&size| size >= old_cell_size)
            .unwrap_or(zoom_levels.len().saturating_sub(1));

        // Move to next/previous level
        let new_index = if delta > 0.0 {
            // Zoom in - decrease cell size (smaller index)
            current_index.saturating_sub(1)
        } else {
            // Zoom out - increase cell size (larger index)
            (current_index + 1).min(zoom_levels.len().saturating_sub(1))
        };

        let new_cell_size = zoom_levels[new_index];

        // Only update if cell size actually changed
        if new_cell_size != old_cell_size {
            // Calculate world position under cursor before zoom
            let old_visible_cells_x = self.window_width as f32 / old_cell_size as f32;
            let old_visible_cells_y = self.window_height as f32 / old_cell_size as f32;

            // Cursor position as fraction of window
            let cursor_frac_x = cursor_x as f32 / self.window_width as f32;
            let cursor_frac_y = cursor_y as f32 / self.window_height as f32;

            // World cell position under cursor
            let world_x_at_cursor = self.viewport.offset_x + cursor_frac_x * old_visible_cells_x;
            let world_y_at_cursor = self.viewport.offset_y + cursor_frac_y * old_visible_cells_y;

            // Apply zoom
            self.current_cell_size = new_cell_size;

            // Calculate new visible cells with new cell size
            let new_visible_cells_x = self.window_width as f32 / new_cell_size as f32;
            let new_visible_cells_y = self.window_height as f32 / new_cell_size as f32;

            // Adjust viewport offset to keep the same world position under cursor
            self.viewport.offset_x = world_x_at_cursor - cursor_frac_x * new_visible_cells_x;
            self.viewport.offset_y = world_y_at_cursor - cursor_frac_y * new_visible_cells_y;

            // Clamp offset_y to not go below 0
            self.viewport.offset_y = self.viewport.offset_y.max(0.0);

            self.mark_viewport_changed();
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface = self.surface.as_ref().unwrap();
        let output = surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update render params every frame to reflect current viewport
        // This provides immediate visual feedback during dragging/resizing
        if self.bind_group.is_some() {
            self.update_render_params();
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Always render CA if we have a valid buffer (even during recomputation)
            // Uncomputed areas will show as black, giving immediate visual feedback
            if self.bind_group.is_some() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl Drop for RenderApp {
    fn drop(&mut self) {
        // Ensure proper cleanup order: drop GPU resources before surface and window
        // This prevents STATUS_ACCESS_VIOLATION on exit

        // Drop all GPU resources first
        self.bind_group = None;
        self.ca_buffer = None;

        // Drop surface configuration before surface
        self.surface_config = None;

        // Drop surface before window to avoid use-after-free
        if let Some(surface) = self.surface.take() {
            drop(surface);
        }

        // Finally drop window
        self.window = None;
    }
}

impl ApplicationHandler for RenderApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            self.init_window(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Check if viewport reset was requested from web (via JavaScript)
                #[cfg(target_arch = "wasm32")]
                {
                    use std::sync::atomic::Ordering;
                    if crate::web::RESET_VIEWPORT_REQUESTED.load(Ordering::SeqCst) {
                        self.reset_viewport();
                        crate::web::RESET_VIEWPORT_REQUESTED.store(false, Ordering::SeqCst);
                    }
                }

                // Check if debounce period has elapsed and recompute if needed
                self.check_debounce_and_recompute();

                match self.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        let size = self.window.as_ref().unwrap().inner_size();
                        if let Some(config) = &mut self.surface_config {
                            config.width = size.width;
                            config.height = size.height;
                            self.surface.as_ref().unwrap().configure(&self.device, config);
                        }
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        event_loop.exit();
                    }
                    Err(e) => eprintln!("Render error: {:?}", e),
                }
            }
            WindowEvent::Resized(physical_size) => {
                // Update surface configuration for new window size
                if let Some(config) = &mut self.surface_config {
                    config.width = physical_size.width;
                    config.height = physical_size.height;
                    self.surface.as_ref().unwrap().configure(&self.device, config);
                }

                // Detect which edge(s) are being resized by tracking window position
                if let Some(window) = &self.window {
                    if let Ok(outer_position) = window.outer_position() {
                        let new_pos = (outer_position.x, outer_position.y);

                        if let Some(old_pos) = self.window_position {
                            let old_width = self.window_width;
                            let old_height = self.window_height;
                            let new_width = physical_size.width;
                            let new_height = physical_size.height;

                            // Calculate visible cells
                            let old_visible_x = old_width as f32 / self.current_cell_size as f32;
                            let new_visible_x = new_width as f32 / self.current_cell_size as f32;
                            let old_visible_y = old_height as f32 / self.current_cell_size as f32;
                            let new_visible_y = new_height as f32 / self.current_cell_size as f32;

                            // If window position changed, we're resizing from left or top
                            if new_pos.0 != old_pos.0 {
                                // Left edge moved - adjust offset to keep right edge fixed
                                let old_right = self.viewport.offset_x + old_visible_x;
                                self.viewport.offset_x = old_right - new_visible_x;
                            }

                            if new_pos.1 != old_pos.1 {
                                // Top edge moved - adjust offset to keep bottom edge fixed
                                let old_bottom = self.viewport.offset_y + old_visible_y;
                                self.viewport.offset_y = old_bottom - new_visible_y;
                                // Clamp to not go below 0
                                self.viewport.offset_y = self.viewport.offset_y.max(0.0);
                            }
                        }

                        self.window_position = Some(new_pos);
                    }
                }

                // Update window dimensions immediately
                // The shader will handle rendering the old buffer at the new size
                // without stretching (showing black for newly exposed areas)
                self.window_width = physical_size.width;
                self.window_height = physical_size.height;

                // Mark for recompute - after debounce we'll compute for new viewport
                self.mark_viewport_changed();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 10.0,
                };

                // Use tracked cursor position
                let (cursor_x, cursor_y) = self.cursor_position;
                self.handle_zoom(delta_y, cursor_x, cursor_y);
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Track cursor position
                self.cursor_position = (position.x, position.y);

                if let Some(ref mut drag) = self.drag_state {
                    if drag.active {
                        // Calculate delta in screen pixels
                        let delta_x = position.x - drag.start_x;
                        let delta_y = position.y - drag.start_y;

                        // Convert to cell delta
                        let visible_cells_x = ((self.window_width as f32 / self.current_cell_size as f32) / self.viewport.zoom) as f32;
                        let visible_cells_y = ((self.window_height as f32 / self.current_cell_size as f32) / self.viewport.zoom) as f32;

                        let delta_cells_x = -(delta_x as f32 / self.window_width as f32) * visible_cells_x;
                        let delta_cells_y = -(delta_y as f32 / self.window_height as f32) * visible_cells_y;

                        // Apply offset from drag start position
                        self.viewport.offset_x = drag.viewport_at_start.offset_x + delta_cells_x;
                        self.viewport.offset_y = drag.viewport_at_start.offset_y + delta_cells_y;

                        // Clamp offset_y to not go below 0
                        self.viewport.offset_y = self.viewport.offset_y.max(0.0);

                        self.mark_viewport_changed();
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    match state {
                        winit::event::ElementState::Pressed => {
                            // Start drag - change cursor to hand
                            if let Some(window) = &self.window {
                                window.set_cursor(winit::window::Cursor::Icon(winit::window::CursorIcon::Grabbing));
                            }

                            let (pos_x, pos_y) = self.cursor_position;

                            self.drag_state = Some(DragState {
                                active: true,
                                start_x: pos_x,
                                start_y: pos_y,
                                viewport_at_start: self.viewport.clone(),
                            });
                        }
                        winit::event::ElementState::Released => {
                            // End drag - restore default cursor
                            if let Some(window) = &self.window {
                                window.set_cursor(winit::window::Cursor::Icon(winit::window::CursorIcon::Default));
                            }

                            if let Some(ref mut drag) = self.drag_state {
                                drag.active = false;
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        match keycode {
                            KeyCode::F11 => {
                                // Toggle fullscreen
                                if let Some(window) = &self.window {
                                    let is_fullscreen = window.fullscreen().is_some();
                                    window.set_fullscreen(if is_fullscreen {
                                        None
                                    } else {
                                        Some(winit::window::Fullscreen::Borderless(None))
                                    });
                                }
                            }
                            KeyCode::Escape => {
                                // Exit fullscreen or close
                                if let Some(window) = &self.window {
                                    if window.fullscreen().is_some() {
                                        window.set_fullscreen(None);
                                    } else {
                                        println!("Escape pressed, exiting...");
                                        event_loop.exit();
                                    }
                                }
                            }
                            KeyCode::Digit0 | KeyCode::Numpad0 => {
                                self.reset_viewport();
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
