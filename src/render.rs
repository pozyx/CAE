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

use crate::{cache::TileCache, compute, constants, Config};
use crate::{log_info, log_warn, log_error};

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

// Touch state for touch gestures (mobile and desktop touchscreens)
struct TouchState {
    // Single touch for panning
    single_touch: Option<(u64, f64, f64)>,  // (touch_id, x, y)
    // Two touches for pinch zoom
    touch1: Option<(u64, f64, f64)>,  // (touch_id, x, y)
    touch2: Option<(u64, f64, f64)>,  // (touch_id, x, y)
    initial_distance: Option<f32>,
    initial_cell_size: Option<u32>,
    viewport_at_pinch_start: Option<Viewport>,
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
    #[allow(dead_code)]  // Only used on web target
    url_params_applied: bool,  // Track if URL parameters were applied (web only)
    touch_state: TouchState,  // Touch gesture state (mobile and desktop touchscreens)

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
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
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
        log_info!("Using GPU: {} ({:?})", info.name, info.backend);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to create device");

        // Set up error handling for GPU device
        #[cfg(target_arch = "wasm32")]
        device.on_uncaptured_error(Box::new(|error| {
            log::error!("WebGPU uncaptured error: {:?}", error);
            // Don't panic - just log the error and continue
            // This prevents the "unreachable" crash when GPU operations fail
        }));

        // Initial window dimensions
        let window_width = config.width;
        let window_height = config.height;

        log_info!("Initial window size: {}x{} pixels, cell size: {}px",
            window_width, window_height, constants::DEFAULT_CELL_SIZE);

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
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
            visible_width: window_width / constants::DEFAULT_CELL_SIZE,
            visible_height: window_height / constants::DEFAULT_CELL_SIZE,
            simulated_width: window_width / constants::DEFAULT_CELL_SIZE,
            padding_left: 0,
            cell_size: constants::DEFAULT_CELL_SIZE,
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

        let cell_size = constants::DEFAULT_CELL_SIZE;

        let cache_tiles = config.cache_tiles;
        let tile_size = config.tile_size;

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
                let mut vp = Viewport::new();
                // Origin (0, 0) means: center horizontally, top vertically
                // So offset_x should be negative half of visible width to center the origin
                let visible_cells_x = window_width as f32 / cell_size as f32;
                vp.offset_x = -visible_cells_x / 2.0;
                vp.offset_y = 0.0;
                vp
            },
            buffer_viewport: Viewport::new(),
            drag_state: None,
            last_viewport_change: None,
            needs_recompute: true,
            cursor_position: (window_width as f64 / 2.0, window_height as f64 / 2.0),
            url_params_applied: false,
            touch_state: TouchState {
                single_touch: None,
                touch1: None,
                touch2: None,
                initial_distance: None,
                initial_cell_size: None,
                viewport_at_pinch_start: None,
            },

            window_width,
            window_height,
            current_cell_size: cell_size,

            cache: if cache_tiles > 0 {
                Some(TileCache::new(cache_tiles, tile_size))
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
        // Apply initial viewport from URL parameters if set (web only)
        #[cfg(target_arch = "wasm32")]
        {
            use std::sync::atomic::Ordering;
            if crate::web::INITIAL_VIEWPORT_SET.load(Ordering::SeqCst) {
                let center_x = *crate::web::INITIAL_OFFSET_X.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let initial_y = *crate::web::INITIAL_OFFSET_Y.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let initial_cell_size = crate::web::INITIAL_CELL_SIZE.load(Ordering::SeqCst);

                // Convert from "center position" (URL vx) to internal offset
                // offset_x = world position at LEFT edge of screen
                // center_x = world position at CENTER of screen
                // So: offset_x = center_x - (visible_width / 2)
                let visible_cells_x = self.window_width as f32 / initial_cell_size as f32;
                self.viewport.offset_x = center_x - (visible_cells_x / 2.0);
                self.viewport.offset_y = initial_y;
                self.current_cell_size = initial_cell_size;

                // Update viewport state globals to reflect the URL parameters
                // This ensures the URL updater gets the correct values
                let visible_cells_x = self.window_width as f32 / self.current_cell_size as f32;
                let url_center_x = self.viewport.offset_x + (visible_cells_x / 2.0);
                *crate::web::VIEWPORT_OFFSET_X.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = url_center_x;
                *crate::web::VIEWPORT_OFFSET_Y.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = self.viewport.offset_y;
                crate::web::VIEWPORT_CELL_SIZE.store(self.current_cell_size, Ordering::SeqCst);

                // Mark that URL parameters were applied
                self.url_params_applied = true;

                // Clear the flag so we don't reapply on subsequent inits
                crate::web::INITIAL_VIEWPORT_SET.store(false, Ordering::SeqCst);
            }
        }

        let mut window_attributes = Window::default_attributes()
            .with_title(format!("CAE - Cellular Automaton Engine | Rule {}", self.config.rule));

        // On desktop, set the window size and constraints. On web, don't - let it use the canvas's existing size
        #[cfg(not(target_arch = "wasm32"))]
        {
            use winit::dpi::PhysicalSize;

            // Set initial size
            window_attributes = window_attributes.with_inner_size(PhysicalSize::new(self.window_width, self.window_height));

            // Set min/max size constraints based on validation limits (500-8192)
            // This allows resizing but keeps it within valid bounds
            let min_size = PhysicalSize::new(500u32, 500u32);
            let max_size = PhysicalSize::new(8192u32, 8192u32);
            window_attributes = window_attributes
                .with_min_inner_size(min_size)
                .with_max_inner_size(max_size);
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

        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log_error!("Failed to create window: {:?}", e);
                panic!("Cannot create window: {:?}", e);
            }
        };

        // Update actual window dimensions (may differ from requested if fullscreen)
        let actual_size = window.inner_size();

        // On web, if window reports size 0, fall back to config dimensions
        if actual_size.width > 0 && actual_size.height > 0 {
            self.window_width = actual_size.width;
            self.window_height = actual_size.height;
        } else {
            log_warn!("Window reported size {}x{}, using config dimensions {}x{}",
                actual_size.width, actual_size.height, self.window_width, self.window_height);
        }

        // Create surface
        let surface = match self.instance.create_surface(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                log_error!("Failed to create surface: {:?}", e);
                panic!("Cannot create surface: {:?}", e);
            }
        };

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

        // Choose best present mode for smooth rendering
        // Prefer Mailbox (triple buffering) for low latency smooth panning
        // Fall back to AutoVsync, then Fifo (VSync)
        #[cfg(target_arch = "wasm32")]
        log::info!("Available present modes: {:?}", surface_caps.present_modes);

        let present_mode = surface_caps.present_modes.iter()
            .copied()
            .find(|mode| matches!(mode, wgpu::PresentMode::Mailbox))
            .or_else(|| surface_caps.present_modes.iter()
                .copied()
                .find(|mode| matches!(mode, wgpu::PresentMode::AutoVsync)))
            .unwrap_or(wgpu::PresentMode::Fifo);

        log_info!("Using present mode: {:?}", present_mode);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.window_width,
            height: self.window_height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&self.device, &config);

        self.window = Some(window.clone());
        self.surface = Some(surface);
        self.surface_config = Some(config);

        // Store window reference for web reset_viewport function
        #[cfg(target_arch = "wasm32")]
        crate::web::set_window_ref(window.clone());

        // Now compute the CA
        self.compute_ca();

        // Request initial redraw for on-demand rendering
        window.request_redraw();
    }

    fn compute_ca(&mut self) {
        log_info!("Computing cellular automaton...");

        // Calculate visible cells based on window size, cell size, and zoom
        // Use ceil to include partial cells at the edges
        let visible_cells_x = ((self.window_width as f32 / self.current_cell_size as f32) / self.viewport.zoom).ceil() as u32;
        let visible_cells_y = ((self.window_height as f32 / self.current_cell_size as f32) / self.viewport.zoom).ceil() as u32;

        // Safety: limit maximum buffer dimensions to prevent GPU issues
        if self.current_cell_size < constants::MIN_CELL_SIZE {
            log_warn!("Cell size {} is too small (minimum {})",
                self.current_cell_size, constants::MIN_CELL_SIZE);
            log_warn!("Skipping computation to prevent GPU instability.");
            return;
        }

        if visible_cells_x > constants::MAX_CELLS_X || visible_cells_y > constants::MAX_CELLS_Y {
            log_warn!("Requested dimensions {}x{} exceed safety limits ({}x{})",
                visible_cells_x, visible_cells_y, constants::MAX_CELLS_X, constants::MAX_CELLS_Y);
            log_warn!("Skipping computation to prevent GPU instability.");
            return;
        }

        // Also check total cell count (width * height * padding factor)
        let total_cells = (visible_cells_x as u64 * 3) * visible_cells_y as u64;  // 3x for padding

        if total_cells > constants::MAX_TOTAL_CELLS {
            log_warn!("Total cell count {} exceeds limit {}",
                total_cells, constants::MAX_TOTAL_CELLS);
            log_warn!("Skipping computation to prevent GPU instability.");
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

        log_info!("Viewport - offset: ({:.1}, {:.1}), zoom: {:.2}",
            self.viewport.offset_x, clamped_offset_y, self.viewport.zoom);
        log_info!("Visible cells: {}x{}, iterations: {}", visible_cells_x, visible_cells_y, iterations);

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

        log_info!("CA result - Simulated: {}x{}, Visible: {}x{}, Padding: {}",
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

        log_info!("Computation complete! (zero-copy GPU rendering)");
    }

    fn mark_viewport_changed(&mut self) {
        self.last_viewport_change = Some(Instant::now());
        self.needs_recompute = true;

        // Request redraw for on-demand rendering
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        // Note: We don't update viewport state globals here anymore
        // They are only updated when user explicitly pans/zooms via update_viewport_state_for_url()
    }

    #[cfg(target_arch = "wasm32")]
    fn update_viewport_state_for_url(&mut self) {
        // Update viewport state for JavaScript URL updates
        // This should only be called when user explicitly pans/zooms
        use std::sync::atomic::Ordering;

        // Convert internal offset to center position for URL
        // offset_x = world position at LEFT edge
        // center_x (for URL) = world position at CENTER
        // So: center_x = offset_x + (visible_width / 2)
        let visible_cells_x = self.window_width as f32 / self.current_cell_size as f32;
        let center_x = self.viewport.offset_x + (visible_cells_x / 2.0);

        *crate::web::VIEWPORT_OFFSET_X.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = center_x;
        *crate::web::VIEWPORT_OFFSET_Y.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = self.viewport.offset_y;
        crate::web::VIEWPORT_CELL_SIZE.store(self.current_cell_size, Ordering::SeqCst);
    }

    fn update_render_params(&mut self) {
        // Throttle params updates to ~60 FPS to reduce GPU load
        // This prevents excessive buffer writes during rapid viewport changes
        if let Some(last_update) = self.last_params_update {
            if last_update.elapsed() < Duration::from_millis(constants::RENDER_PARAMS_THROTTLE_MS) {
                return;  // Skip update if less than throttle time since last update (~60 FPS)
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
        // Reset viewport to initial state (origin at center horizontally, top vertically)
        log_info!("Resetting viewport to initial state...");
        self.current_cell_size = constants::DEFAULT_CELL_SIZE;
        self.viewport.zoom = 1.0;

        // Origin (0, 0) means: center horizontally, top vertically
        let visible_cells_x = self.window_width as f32 / self.current_cell_size as f32;
        self.viewport.offset_x = -visible_cells_x / 2.0;
        self.viewport.offset_y = 0.0;

        self.needs_recompute = true;
        self.last_viewport_change = Some(Instant::now());

        // Request redraw for on-demand rendering
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        // Update viewport state for JavaScript URL updates (web only)
        #[cfg(target_arch = "wasm32")]
        self.update_viewport_state_for_url();
    }

    fn handle_zoom(&mut self, delta: f32, cursor_x: f64, cursor_y: f64) {
        // Hardcoded zoom limits
        // Zoom > 1.0 means zoomed IN (cells appear bigger)
        // Zoom < 1.0 means zoomed OUT (cells appear smaller)
        // zoom_factor = current_cell_size / base_cell_size
        let base_cell_size = constants::DEFAULT_CELL_SIZE;
        let min_cell_size = (base_cell_size as f32 * constants::ZOOM_MIN).max(1.0) as u32;
        let max_cell_size = (base_cell_size as f32 * constants::ZOOM_MAX) as u32;

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
            // Zoom in - increase cell size (larger index)
            (current_index + 1).min(zoom_levels.len().saturating_sub(1))
        } else {
            // Zoom out - decrease cell size (smaller index)
            current_index.saturating_sub(1)
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

            // Update URL state for web (only after user interaction)
            #[cfg(target_arch = "wasm32")]
            self.update_viewport_state_for_url();
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Defensive: handle case where surface is None (GPU context loss on window restore)
        let surface = match self.surface.as_ref() {
            Some(s) => s,
            None => {
                #[cfg(target_arch = "wasm32")]
                log::warn!("Render called but surface is None (GPU context lost)");
                return Err(wgpu::SurfaceError::Lost);
            }
        };
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
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Always render CA if we have a valid buffer (even during recomputation)
            // Uncomputed areas will show as black, giving immediate visual feedback
            if let Some(bind_group) = &self.bind_group {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
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

impl RenderApp {
    fn handle_touch(&mut self, touch: winit::event::Touch) {
        use winit::event::TouchPhase;

        match touch.phase {
            TouchPhase::Started => {
                // First touch
                if self.touch_state.touch1.is_none() {
                    self.touch_state.touch1 = Some((touch.id, touch.location.x, touch.location.y));
                    // Start single-touch pan
                    self.touch_state.single_touch = Some((touch.id, touch.location.x, touch.location.y));
                    self.drag_state = Some(DragState {
                        active: true,
                        start_x: touch.location.x,
                        start_y: touch.location.y,
                        viewport_at_start: self.viewport.clone(),
                    });
                }
                // Second touch - start pinch zoom
                else if self.touch_state.touch2.is_none() {
                    self.touch_state.touch2 = Some((touch.id, touch.location.x, touch.location.y));
                    // Cancel single touch pan
                    self.touch_state.single_touch = None;
                    self.drag_state = None;

                    // Calculate initial distance between touches
                    if let (Some((_, x1, y1)), Some((_, x2, y2))) = (self.touch_state.touch1, self.touch_state.touch2) {
                        let dx = x2 - x1;
                        let dy = y2 - y1;
                        let distance = ((dx * dx + dy * dy) as f32).sqrt();
                        self.touch_state.initial_distance = Some(distance);
                        self.touch_state.initial_cell_size = Some(self.current_cell_size);
                        self.touch_state.viewport_at_pinch_start = Some(self.viewport.clone());
                    }
                }
            }
            TouchPhase::Moved => {
                // Single touch pan
                if let Some((id, _, _)) = self.touch_state.single_touch {
                    if touch.id == id {
                        // Update pan - use existing drag logic
                        if let Some(ref mut drag) = self.drag_state {
                            let delta_x = touch.location.x - drag.start_x;
                            let delta_y = touch.location.y - drag.start_y;

                            let visible_cells_x = self.window_width as f32 / self.current_cell_size as f32;
                            let visible_cells_y = self.window_height as f32 / self.current_cell_size as f32;

                            let delta_cells_x = -(delta_x as f32 / self.window_width as f32) * visible_cells_x;
                            let delta_cells_y = -(delta_y as f32 / self.window_height as f32) * visible_cells_y;

                            self.viewport.offset_x = drag.viewport_at_start.offset_x + delta_cells_x;
                            self.viewport.offset_y = drag.viewport_at_start.offset_y + delta_cells_y;
                            self.viewport.offset_y = self.viewport.offset_y.max(0.0);

                            self.mark_viewport_changed();
                            #[cfg(target_arch = "wasm32")]
                            self.update_viewport_state_for_url();
                        }
                    }
                }
                // Pinch zoom
                else if self.touch_state.touch1.is_some() && self.touch_state.touch2.is_some() {
                    // Update touch positions
                    if let Some((id1, ref mut x1, ref mut y1)) = self.touch_state.touch1 {
                        if touch.id == id1 {
                            *x1 = touch.location.x;
                            *y1 = touch.location.y;
                        }
                    }
                    if let Some((id2, ref mut x2, ref mut y2)) = self.touch_state.touch2 {
                        if touch.id == id2 {
                            *x2 = touch.location.x;
                            *y2 = touch.location.y;
                        }
                    }

                    // Calculate current distance and zoom
                    if let (Some((_, x1, y1)), Some((_, x2, y2))) = (self.touch_state.touch1, self.touch_state.touch2) {
                        let dx = x2 - x1;
                        let dy = y2 - y1;
                        let current_distance = ((dx * dx + dy * dy) as f32).sqrt();

                        if let (Some(initial_distance), Some(initial_cell_size), Some(ref _viewport_start)) =
                            (self.touch_state.initial_distance, self.touch_state.initial_cell_size, &self.touch_state.viewport_at_pinch_start) {

                            // Calculate zoom factor
                            let zoom_factor = current_distance / initial_distance;
                            let new_cell_size = (initial_cell_size as f32 * zoom_factor).max(1.0).min(500.0) as u32;

                            // Clamp to available zoom levels
                            let min_cell_size = (constants::DEFAULT_CELL_SIZE as f32 * constants::ZOOM_MIN).max(1.0) as u32;
                            let max_cell_size = (constants::DEFAULT_CELL_SIZE as f32 * constants::ZOOM_MAX) as u32;
                            let clamped_cell_size = new_cell_size.clamp(min_cell_size, max_cell_size);

                            // Find nearest zoom level
                            let zoom_levels: Vec<u32> = {
                                let mut levels = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300, 400, 500];
                                levels.retain(|&z| z >= min_cell_size && z <= max_cell_size);
                                levels
                            };

                            let new_cell_size = zoom_levels.iter()
                                .min_by_key(|&&level| ((level as i32) - (clamped_cell_size as i32)).abs())
                                .copied()
                                .unwrap_or(clamped_cell_size);

                            if new_cell_size != self.current_cell_size {
                                // Calculate pinch center
                                let center_x = (x1 + x2) / 2.0;
                                let center_y = (y1 + y2) / 2.0;

                                // Calculate world position at pinch center with old cell size
                                let old_visible_x = self.window_width as f32 / self.current_cell_size as f32;
                                let old_visible_y = self.window_height as f32 / self.current_cell_size as f32;
                                let cursor_frac_x = center_x as f32 / self.window_width as f32;
                                let cursor_frac_y = center_y as f32 / self.window_height as f32;
                                let world_x_at_cursor = self.viewport.offset_x + cursor_frac_x * old_visible_x;
                                let world_y_at_cursor = self.viewport.offset_y + cursor_frac_y * old_visible_y;

                                // Update cell size
                                self.current_cell_size = new_cell_size;

                                // Adjust viewport to keep world position at cursor fixed
                                let new_visible_x = self.window_width as f32 / new_cell_size as f32;
                                let new_visible_y = self.window_height as f32 / self.current_cell_size as f32;
                                self.viewport.offset_x = world_x_at_cursor - cursor_frac_x * new_visible_x;
                                self.viewport.offset_y = world_y_at_cursor - cursor_frac_y * new_visible_y;
                                self.viewport.offset_y = self.viewport.offset_y.max(0.0);

                                self.mark_viewport_changed();
                                #[cfg(target_arch = "wasm32")]
                                self.update_viewport_state_for_url();
                            }
                        }
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                // Remove the ended touch
                if let Some((id1, _, _)) = self.touch_state.touch1 {
                    if touch.id == id1 {
                        self.touch_state.touch1 = self.touch_state.touch2.take();
                        self.touch_state.touch2 = None;
                    }
                }
                if let Some((id2, _, _)) = self.touch_state.touch2 {
                    if touch.id == id2 {
                        self.touch_state.touch2 = None;
                    }
                }

                // Clear single touch
                if let Some((id, _, _)) = self.touch_state.single_touch {
                    if touch.id == id {
                        self.touch_state.single_touch = None;
                        self.drag_state = None;
                    }
                }

                // Reset pinch state if no touches remain
                if self.touch_state.touch1.is_none() {
                    self.touch_state.initial_distance = None;
                    self.touch_state.initial_cell_size = None;
                    self.touch_state.viewport_at_pinch_start = None;
                }

                // If one touch remains after pinch, restart pan
                if self.touch_state.touch1.is_some() && self.touch_state.touch2.is_none() {
                    if let Some((id, x, y)) = self.touch_state.touch1 {
                        self.touch_state.single_touch = Some((id, x, y));
                        self.drag_state = Some(DragState {
                            active: true,
                            start_x: x,
                            start_y: y,
                            viewport_at_start: self.viewport.clone(),
                        });
                    }
                }
            }
        }
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
                log_info!("Close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Skip rendering when window is minimized (width or height = 0)
                // This prevents "Render error: Outdated" spam in logs
                if self.window_width == 0 || self.window_height == 0 {
                    return;
                }

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
                        // Surface was lost (GPU context loss, window minimize/restore, etc.)
                        // Try to reconfigure the surface
                        #[cfg(target_arch = "wasm32")]
                        log::warn!("Surface lost, attempting to reconfigure");

                        if let (Some(window), Some(surface), Some(config)) =
                            (&self.window, &self.surface, &mut self.surface_config) {
                            let size = window.inner_size();
                            config.width = size.width;
                            config.height = size.height;
                            surface.configure(&self.device, &config);

                            #[cfg(target_arch = "wasm32")]
                            log::info!("Surface reconfigured successfully");
                        } else {
                            #[cfg(target_arch = "wasm32")]
                            log::error!("Cannot reconfigure surface: window, surface, or config is None");
                        }
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        #[cfg(target_arch = "wasm32")]
                        log::error!("Out of GPU memory, exiting");
                        event_loop.exit();
                    }
                    Err(e) => {
                        log_warn!("Render error: {:?}", e);
                    }
                }
            }
            WindowEvent::Resized(physical_size) => {
                // On web, initial resize event may occur after window creation (e.g., high-DPI displays)
                // Recalculate viewport offset to maintain the correct center position
                #[cfg(target_arch = "wasm32")]
                {
                    if self.last_viewport_change.is_none() {
                        let old_width = self.window_width;
                        let new_width = physical_size.width;

                        if old_width != new_width {
                            if !self.url_params_applied {
                                // First resize on web with no URL params - recalculate offset to maintain centered origin
                                let visible_cells_x = new_width as f32 / self.current_cell_size as f32;
                                self.viewport.offset_x = -visible_cells_x / 2.0;
                            } else {
                                // First resize on web WITH URL params - recalculate offset to maintain center position from URL
                                // Calculate current center position
                                let old_visible_x = old_width as f32 / self.current_cell_size as f32;
                                let center_x = self.viewport.offset_x + (old_visible_x / 2.0);

                                // Recalculate offset for new width to maintain same center
                                let new_visible_x = new_width as f32 / self.current_cell_size as f32;
                                self.viewport.offset_x = center_x - (new_visible_x / 2.0);
                            }
                        }
                    }
                }

                // Update window dimensions immediately (even if minimized)
                // This ensures RedrawRequested knows not to render
                self.window_width = physical_size.width;
                self.window_height = physical_size.height;

                // Skip surface reconfiguration when window is minimized (width or height = 0)
                // This prevents wgpu errors about zero-sized surfaces
                if physical_size.width == 0 || physical_size.height == 0 {
                    return;
                }

                // Update surface configuration for new window size
                if let (Some(config), Some(surface)) = (&mut self.surface_config, &self.surface) {
                    config.width = physical_size.width;
                    config.height = physical_size.height;
                    surface.configure(&self.device, config);
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

                        // Update URL state for web (only after user interaction)
                        #[cfg(target_arch = "wasm32")]
                        self.update_viewport_state_for_url();
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
                                        log_info!("Escape pressed, exiting...");
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
            WindowEvent::Touch(touch) => {
                self.handle_touch(touch);
            }
            _ => {}
        }
        // Note: We no longer unconditionally request redraw here.
        // Redraws are requested only when viewport changes (mark_viewport_changed)
        // This implements on-demand rendering to prevent continuous GPU usage.
    }
}
