#include "render.h"

#include <algorithm>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <vector>

#include <cuda_runtime.h>
#include <cuda_gl_interop.h>

#ifdef _WIN32
#define NOMINMAX
#define GLFW_EXPOSE_NATIVE_WIN32
#include <GLFW/glfw3native.h>
#include <windows.h>
#include <dwmapi.h>
#include <ShObjIdl.h>
#else
#include <unistd.h>
#endif

namespace cae {

#ifdef _WIN32
// Forward declarations for Win32 touch WndProc (defined below)
static WNDPROC s_original_wndproc;
static RenderApp* s_touch_app;
static LRESULT CALLBACK touch_wndproc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);

// Notify the Windows shell that a window is entering/leaving fullscreen.
// This makes the taskbar auto-hide and tells the DWM compositor to handle
// the transition properly (prevents dimming artifacts on secondary monitors).
static void taskbar_mark_fullscreen(HWND hwnd, bool fullscreen) {
    ITaskbarList2* taskbar = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TaskbarList, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_ITaskbarList2, reinterpret_cast<void**>(&taskbar)))) {
        taskbar->HrInit();
        taskbar->MarkFullscreenWindow(hwnd, fullscreen ? TRUE : FALSE);
        taskbar->Release();
    }
}
#endif

// Returns the monitor whose area contains the center of the given window,
// or the primary monitor as fallback.
static GLFWmonitor* get_current_monitor(GLFWwindow* window) {
    int wx, wy, ww, wh;
    glfwGetWindowPos(window, &wx, &wy);
    glfwGetWindowSize(window, &ww, &wh);
    int cx = wx + ww / 2;
    int cy = wy + wh / 2;

    int count;
    GLFWmonitor** monitors = glfwGetMonitors(&count);
    for (int i = 0; i < count; ++i) {
        int mx, my;
        glfwGetMonitorPos(monitors[i], &mx, &my);
        const GLFWvidmode* mode = glfwGetVideoMode(monitors[i]);
        if (cx >= mx && cx < mx + mode->width &&
            cy >= my && cy < my + mode->height) {
            return monitors[i];
        }
    }
    return glfwGetPrimaryMonitor();
}

// --- Static GLFW callback trampolines ---

static void glfw_error_callback(int error, const char* description) {
    std::cerr << "GLFW Error " << error << ": " << description << std::endl;
}

static void glfw_framebuffer_size_callback(GLFWwindow* window, int width, int height) {
    auto* app = static_cast<RenderApp*>(glfwGetWindowUserPointer(window));
    if (app) app->handleResize(width, height);
}

static void glfw_scroll_callback(GLFWwindow* window, double xoffset, double yoffset) {
    auto* app = static_cast<RenderApp*>(glfwGetWindowUserPointer(window));
    if (app) app->handleScroll(xoffset, yoffset);
}

static void glfw_mouse_button_callback(GLFWwindow* window, int button, int action, int mods) {
    auto* app = static_cast<RenderApp*>(glfwGetWindowUserPointer(window));
    if (app) app->handleMouseButton(button, action, mods);
}

static void glfw_cursor_pos_callback(GLFWwindow* window, double xpos, double ypos) {
    auto* app = static_cast<RenderApp*>(glfwGetWindowUserPointer(window));
    if (app) app->handleCursorPos(xpos, ypos);
}

static void glfw_key_callback(GLFWwindow* window, int key, int scancode, int action, int mods) {
    auto* app = static_cast<RenderApp*>(glfwGetWindowUserPointer(window));
    if (app) app->handleKey(key, scancode, action, mods);
}

// --- Path helper: resolve paths relative to executable directory ---

static std::string get_exe_dir() {
#ifdef _WIN32
    char buf[MAX_PATH];
    GetModuleFileNameA(nullptr, buf, MAX_PATH);
    std::string path(buf);
    auto pos = path.find_last_of("\\/");
    return (pos != std::string::npos) ? path.substr(0, pos) : ".";
#else
    char buf[4096];
    ssize_t len = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
    if (len == -1) return ".";
    buf[len] = '\0';
    std::string path(buf);
    auto pos = path.find_last_of('/');
    return (pos != std::string::npos) ? path.substr(0, pos) : ".";
#endif
}

// --- Shader loading helper ---

static std::string load_shader_file(const std::string& path) {
    // Resolve relative to executable directory
    std::string full_path = get_exe_dir() + "/" + path;
    std::ifstream file(full_path);
    if (!file.is_open()) {
        std::cerr << "Error: Could not open shader file: " << full_path << std::endl;
        return "";
    }
    std::stringstream ss;
    ss << file.rdbuf();
    return ss.str();
}

static GLuint compile_shader(GLenum type, const std::string& source) {
    GLuint shader = glCreateShader(type);
    const char* src = source.c_str();
    glShaderSource(shader, 1, &src, nullptr);
    glCompileShader(shader);

    GLint success;
    glGetShaderiv(shader, GL_COMPILE_STATUS, &success);
    if (!success) {
        char log[1024];
        glGetShaderInfoLog(shader, sizeof(log), nullptr, log);
        std::cerr << "Shader compilation error: " << log << std::endl;
        glDeleteShader(shader);
        return 0;
    }
    return shader;
}

static GLuint create_shader_program(const std::string& vert_source, const std::string& frag_source) {
    GLuint vert = compile_shader(GL_VERTEX_SHADER, vert_source);
    GLuint frag = compile_shader(GL_FRAGMENT_SHADER, frag_source);
    if (!vert || !frag) {
        if (vert) glDeleteShader(vert);
        if (frag) glDeleteShader(frag);
        return 0;
    }

    GLuint program = glCreateProgram();
    glAttachShader(program, vert);
    glAttachShader(program, frag);
    glLinkProgram(program);

    GLint success;
    glGetProgramiv(program, GL_LINK_STATUS, &success);
    if (!success) {
        char log[1024];
        glGetProgramInfoLog(program, sizeof(log), nullptr, log);
        std::cerr << "Shader link error: " << log << std::endl;
        glDeleteProgram(program);
        program = 0;
    }

    glDeleteShader(vert);
    glDeleteShader(frag);
    return program;
}

// --- RenderApp implementation ---

RenderApp::RenderApp(const Config& config)
    : config_(config)
    , window_width_(config.width)
    , window_height_(config.height)
    , current_cell_size_(constants::DEFAULT_CELL_SIZE)
{
    // Set initial viewport: center horizontally, top vertically
    float visible_cells_x = static_cast<float>(window_width_) / current_cell_size_;
    viewport_.offset_x = -visible_cells_x / 2.0f;
    viewport_.offset_y = 0.0f;
    viewport_.zoom = 1.0f;

    // Initialize cache if enabled
    if (config_.cache_tiles > 0) {
        cache_.emplace(config_.cache_tiles, config_.tile_size);
    }

    initGLFW();
    initOpenGL();
    initShaders();
    initFullScreenQuad();
}

RenderApp::~RenderApp() {
    // Unregister CUDA-GL interop before deleting GL resources
    if (cuda_ssbo_resource_) {
        cudaGraphicsUnregisterResource(cuda_ssbo_resource_);
        cuda_ssbo_resource_ = nullptr;
    }

    // Cleanup OpenGL resources
    if (ca_ssbo_) glDeleteBuffers(1, &ca_ssbo_);
    if (params_ubo_) glDeleteBuffers(1, &params_ubo_);
    if (vao_) glDeleteVertexArrays(1, &vao_);
    if (vbo_) glDeleteBuffers(1, &vbo_);
    if (ebo_) glDeleteBuffers(1, &ebo_);
    if (shader_program_) glDeleteProgram(shader_program_);

    if (hand_cursor_) glfwDestroyCursor(hand_cursor_);
    if (window_) glfwDestroyWindow(window_);
    glfwTerminate();
}

void RenderApp::initGLFW() {
    glfwSetErrorCallback(glfw_error_callback);

    if (!glfwInit()) {
        std::cerr << "Error: Failed to initialize GLFW" << std::endl;
        std::exit(1);
    }

    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 4);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 5);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);

    window_ = glfwCreateWindow(config_.width, config_.height,
        "CAE - Cellular Automaton Engine", nullptr, nullptr);

    if (!window_) {
        std::cerr << "Error: Failed to create GLFW window" << std::endl;
        glfwTerminate();
        std::exit(1);
    }

    // Set window title with rule number
    std::string title = "CAE - Cellular Automaton Engine | Rule " + std::to_string(config_.rule);
    glfwSetWindowTitle(window_, title.c_str());

    // Set minimum window size
    glfwSetWindowSizeLimits(window_, 500, 500, GLFW_DONT_CARE, GLFW_DONT_CARE);

    glfwMakeContextCurrent(window_);
    glfwSetWindowUserPointer(window_, this);

    // Register callbacks
    glfwSetFramebufferSizeCallback(window_, glfw_framebuffer_size_callback);
    glfwSetScrollCallback(window_, glfw_scroll_callback);
    glfwSetMouseButtonCallback(window_, glfw_mouse_button_callback);
    glfwSetCursorPosCallback(window_, glfw_cursor_pos_callback);
    glfwSetKeyCallback(window_, glfw_key_callback);

    hand_cursor_ = glfwCreateStandardCursor(GLFW_HAND_CURSOR);

    // Store windowed position/size for fullscreen toggle
    glfwGetWindowPos(window_, &windowed_x_, &windowed_y_);
    windowed_width_ = config_.width;
    windowed_height_ = config_.height;

#ifdef _WIN32
    HWND hwnd = glfwGetWin32Window(window_);

    // Respect system dark mode preference for the title bar (DWMWA_USE_IMMERSIVE_DARK_MODE = 20)
    BOOL useDarkMode = TRUE;
    DwmSetWindowAttribute(hwnd, 20, &useDarkMode, sizeof(useDarkMode));

    // Track initial DPI for per-monitor DPI change handling
    current_dpi_ = GetDpiForWindow(hwnd);

    // Register for touch input and subclass WndProc
    RegisterTouchWindow(hwnd, 0);
    s_touch_app = this;
    s_original_wndproc = reinterpret_cast<WNDPROC>(
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, reinterpret_cast<LONG_PTR>(touch_wndproc)));
#endif

    // Apply borderless fullscreen if requested at startup (Win32 style manipulation)
    if (config_.fullscreen) {
        enterFullscreen();
    }
}

void RenderApp::initOpenGL() {
    if (!gladLoadGLLoader(reinterpret_cast<GLADloadproc>(glfwGetProcAddress))) {
        std::cerr << "Error: Failed to initialize GLAD" << std::endl;
        std::exit(1);
    }

    glViewport(0, 0, window_width_, window_height_);
    glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
    glfwSwapInterval(0); // Non-blocking swap (like wgpu Mailbox present mode)

    // Initialize CUDA on the same GPU that OpenGL is using (required for interop)
    unsigned int cuda_gl_device_count = 0;
    int cuda_gl_devices[1];
    cudaError_t err = cudaGLGetDevices(&cuda_gl_device_count, cuda_gl_devices, 1,
                                       cudaGLDeviceListAll);
    if (err == cudaSuccess && cuda_gl_device_count > 0) {
        cudaSetDevice(cuda_gl_devices[0]);
        cudaDeviceProp prop;
        cudaGetDeviceProperties(&prop, cuda_gl_devices[0]);
        std::cout << "Using GPU: " << prop.name << " (OpenGL/CUDA)" << std::endl;
    } else {
        std::cerr << "Error: cudaGLGetDevices failed ("
                  << cudaGetErrorString(err)
                  << "). CUDA-GL interop requires OpenGL and CUDA on the same GPU." << std::endl;
        std::exit(1);
    }

    std::cout << "Initial window size: " << window_width_ << "x" << window_height_
              << " pixels, cell size: " << current_cell_size_ << "px" << std::endl;
}

void RenderApp::initShaders() {
    // Try loading from shaders/ directory relative to executable
    std::string vert_source = load_shader_file("shaders/render.vert");
    std::string frag_source = load_shader_file("shaders/render.frag");

    if (vert_source.empty() || frag_source.empty()) {
        std::cerr << "Error: Failed to load shader files" << std::endl;
        std::exit(1);
    }

    shader_program_ = create_shader_program(vert_source, frag_source);
    if (!shader_program_) {
        std::cerr << "Error: Failed to create shader program" << std::endl;
        std::exit(1);
    }
}

void RenderApp::initFullScreenQuad() {
    // Full-screen quad vertices (matches Rust VERTICES array)
    // Position (x, y) + TexCoords (u, v)
    float vertices[] = {
        // Bottom-left
        -1.0f, -1.0f,  0.0f, 1.0f,
        // Bottom-right
         1.0f, -1.0f,  1.0f, 1.0f,
        // Top-right
         1.0f,  1.0f,  1.0f, 0.0f,
        // Top-left
        -1.0f,  1.0f,  0.0f, 0.0f,
    };

    uint16_t indices[] = { 0, 1, 2, 2, 3, 0 };

    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glGenBuffers(1, &ebo_);

    glBindVertexArray(vao_);

    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(GL_ARRAY_BUFFER, sizeof(vertices), vertices, GL_STATIC_DRAW);

    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo_);
    glBufferData(GL_ELEMENT_ARRAY_BUFFER, sizeof(indices), indices, GL_STATIC_DRAW);

    // Position attribute (location 0)
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 4 * sizeof(float), nullptr);
    glEnableVertexAttribArray(0);

    // TexCoords attribute (location 1)
    glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, 4 * sizeof(float),
                          reinterpret_cast<void*>(2 * sizeof(float)));
    glEnableVertexAttribArray(1);

    glBindVertexArray(0);

    // Create params UBO
    glGenBuffers(1, &params_ubo_);
    glBindBuffer(GL_UNIFORM_BUFFER, params_ubo_);
    glBufferData(GL_UNIFORM_BUFFER, sizeof(RenderParams), nullptr, GL_DYNAMIC_DRAW);
    glBindBuffer(GL_UNIFORM_BUFFER, 0);
}

// --- Core operations ---

void RenderApp::computeCA() {
    std::cout << "Computing cellular automaton..." << std::endl;

    // Calculate visible cells
    float visible_x_f = static_cast<float>(window_width_) / current_cell_size_;
    float visible_y_f = static_cast<float>(window_height_) / current_cell_size_;
    uint32_t visible_cells_x = static_cast<uint32_t>(std::ceil(visible_x_f));
    uint32_t visible_cells_y = static_cast<uint32_t>(std::ceil(visible_y_f));

    // Safety checks
    if (current_cell_size_ < constants::MIN_CELL_SIZE) {
        std::cerr << "Warning: Cell size " << current_cell_size_
                  << " too small, skipping computation" << std::endl;
        return;
    }
    if (visible_cells_x > constants::MAX_CELLS_X || visible_cells_y > constants::MAX_CELLS_Y) {
        std::cerr << "Warning: Dimensions " << visible_cells_x << "x" << visible_cells_y
                  << " exceed limits, skipping" << std::endl;
        return;
    }
    uint64_t total_cells = static_cast<uint64_t>(visible_cells_x) * 3 * visible_cells_y;
    if (total_cells > constants::MAX_TOTAL_CELLS) {
        std::cerr << "Warning: Total cells " << total_cells
                  << " exceeds limit, skipping" << std::endl;
        return;
    }

    float clamped_offset_y = std::max(viewport_.offset_y, 0.0f);
    uint32_t start_generation = static_cast<uint32_t>(clamped_offset_y);
    uint32_t iterations = visible_cells_y;
    int32_t horizontal_offset = static_cast<int32_t>(viewport_.offset_x);

    { // Scoped format: avoid polluting global cout state with fixed/setprecision
        auto old_flags = std::cout.flags();
        auto old_prec = std::cout.precision();
        std::cout << std::fixed << std::setprecision(1)
                  << "Viewport - offset: (" << viewport_.offset_x << ", " << clamped_offset_y
                  << "), zoom: " << std::setprecision(2) << viewport_.zoom << std::endl;
        std::cout.flags(old_flags);
        std::cout.precision(old_prec);
    }
    std::cout << "Visible cells: " << visible_cells_x << "x" << visible_cells_y
              << ", iterations: " << iterations << std::endl;

    // Run CA computation
    CaResult ca_result;
    if (cache_.has_value()) {
        ca_result = run_ca_with_cache(
            config_.rule, start_generation, iterations,
            visible_cells_x, horizontal_offset,
            config_.initial_state, cache_.value());
    } else {
        ca_result = run_ca(
            config_.rule, start_generation, iterations,
            visible_cells_x, horizontal_offset,
            config_.initial_state);
    }

    std::cout << "CA result - Simulated: " << ca_result.simulated_width << "x" << ca_result.height
              << ", Visible: " << ca_result.visible_width << "x" << ca_result.height
              << ", Padding: " << ca_result.padding_left << std::endl;

    // Store buffer metadata
    buffer_simulated_width_ = ca_result.simulated_width;
    buffer_visible_width_ = ca_result.visible_width;
    buffer_visible_height_ = ca_result.height;
    buffer_padding_left_ = ca_result.padding_left;
    buffer_viewport_ = viewport_;

    // Transfer CA data to SSBO via CUDA-GL interop (zero-copy, stays on GPU)
    size_t buf_size = ca_result.buffer_size_bytes;

    // Resize SSBO if needed (unregister old CUDA resource first)
    if (ca_ssbo_ == 0 || ca_ssbo_size_ != buf_size) {
        if (cuda_ssbo_resource_) {
            cudaGraphicsUnregisterResource(cuda_ssbo_resource_);
            cuda_ssbo_resource_ = nullptr;
        }
        if (ca_ssbo_) glDeleteBuffers(1, &ca_ssbo_);
        glGenBuffers(1, &ca_ssbo_);
        glBindBuffer(GL_SHADER_STORAGE_BUFFER, ca_ssbo_);
        glBufferData(GL_SHADER_STORAGE_BUFFER, buf_size, nullptr, GL_DYNAMIC_DRAW);
        glBindBuffer(GL_SHADER_STORAGE_BUFFER, 0);
        ca_ssbo_size_ = buf_size;
        cudaError_t err = cudaGraphicsGLRegisterBuffer(
            &cuda_ssbo_resource_, ca_ssbo_, cudaGraphicsMapFlagsWriteDiscard);
        if (err != cudaSuccess) {
            std::cerr << "cudaGraphicsGLRegisterBuffer failed: "
                      << cudaGetErrorString(err) << std::endl;
        }
    }

    // Map SSBO for CUDA, copy result (device-to-device), unmap
    cudaGraphicsMapResources(1, &cuda_ssbo_resource_, nullptr);
    void* d_ssbo_ptr = nullptr;
    size_t mapped_size = 0;
    cudaGraphicsResourceGetMappedPointer(&d_ssbo_ptr, &mapped_size, cuda_ssbo_resource_);
    cudaMemcpy(d_ssbo_ptr, ca_result.d_buffer, buf_size, cudaMemcpyDeviceToDevice);
    cudaGraphicsUnmapResources(1, &cuda_ssbo_resource_, nullptr);

    // Update render params (before freeing ca_result)
    RenderParams params{};
    params.visible_width = ca_result.visible_width;
    params.visible_height = ca_result.height;
    params.simulated_width = ca_result.simulated_width;
    params.padding_left = ca_result.padding_left;
    params.cell_size = current_cell_size_;
    params.window_width = window_width_;
    params.window_height = window_height_;
    params.viewport_offset_x = static_cast<int32_t>(viewport_.offset_x);
    params.viewport_offset_y = static_cast<int32_t>(viewport_.offset_y);
    params.buffer_offset_x = static_cast<int32_t>(viewport_.offset_x);
    params.buffer_offset_y = static_cast<int32_t>(viewport_.offset_y);
    params._padding = 0;

    // Free the computation result buffer (data is now in SSBO)
    free_ca_result(ca_result);

    glBindBuffer(GL_UNIFORM_BUFFER, params_ubo_);
    glBufferSubData(GL_UNIFORM_BUFFER, 0, sizeof(RenderParams), &params);
    glBindBuffer(GL_UNIFORM_BUFFER, 0);

    needs_recompute_ = false;

    std::cout << "Computation complete! (zero-copy GPU rendering)" << std::endl;
}

void RenderApp::render() {
    if (window_width_ == 0 || window_height_ == 0) return;
    if (ca_ssbo_ == 0) return;

    updateRenderParams();

    glClear(GL_COLOR_BUFFER_BIT);
    glUseProgram(shader_program_);

    glBindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, ca_ssbo_);
    glBindBufferBase(GL_UNIFORM_BUFFER, 1, params_ubo_);

    glBindVertexArray(vao_);
    glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_SHORT, nullptr);
    glBindVertexArray(0);

    glfwSwapBuffers(window_);
}

void RenderApp::updateRenderParams() {
    // Throttle to ~60 FPS
    auto now = std::chrono::steady_clock::now();
    if (last_params_update_.has_value()) {
        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            now - last_params_update_.value()).count();
        if (static_cast<uint64_t>(elapsed) < constants::RENDER_PARAMS_THROTTLE_MS) {
            return;
        }
    }

    RenderParams params{};
    params.visible_width = (window_width_ + current_cell_size_ - 1) / current_cell_size_;
    params.visible_height = (window_height_ + current_cell_size_ - 1) / current_cell_size_;
    params.simulated_width = buffer_simulated_width_;
    params.padding_left = buffer_padding_left_;
    params.cell_size = current_cell_size_;
    params.window_width = window_width_;
    params.window_height = window_height_;
    params.viewport_offset_x = static_cast<int32_t>(viewport_.offset_x);
    params.viewport_offset_y = static_cast<int32_t>(viewport_.offset_y);
    params.buffer_offset_x = static_cast<int32_t>(buffer_viewport_.offset_x);
    params.buffer_offset_y = static_cast<int32_t>(buffer_viewport_.offset_y);
    params._padding = 0;

    glBindBuffer(GL_UNIFORM_BUFFER, params_ubo_);
    glBufferSubData(GL_UNIFORM_BUFFER, 0, sizeof(RenderParams), &params);
    glBindBuffer(GL_UNIFORM_BUFFER, 0);

    last_params_update_ = now;
}

void RenderApp::checkDebounceAndRecompute() {
    if (!last_viewport_change_.has_value()) return;
    if (!needs_recompute_) return;

    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - last_viewport_change_.value()).count();

    if (static_cast<uint64_t>(elapsed) >= config_.debounce_ms) {
        computeCA();
        last_viewport_change_.reset();
    }
}

void RenderApp::markViewportChanged() {
    last_viewport_change_ = std::chrono::steady_clock::now();
    needs_recompute_ = true;
    glfwPostEmptyEvent(); // Wake up the event loop
}

void RenderApp::resetViewport() {
    std::cout << "Resetting viewport to initial state..." << std::endl;
    current_cell_size_ = constants::DEFAULT_CELL_SIZE;
    viewport_.zoom = 1.0f;
    float visible_cells_x = static_cast<float>(window_width_) / current_cell_size_;
    viewport_.offset_x = -visible_cells_x / 2.0f;
    viewport_.offset_y = 0.0f;
    needs_recompute_ = true;
    last_viewport_change_ = std::chrono::steady_clock::now();
    glfwPostEmptyEvent();
}

// --- Zoom ---

std::vector<uint32_t> RenderApp::generateZoomLevels() const {
    uint32_t base = constants::DEFAULT_CELL_SIZE;
    uint32_t min_cs = static_cast<uint32_t>(std::max(base * constants::ZOOM_MIN, 1.0f));
    uint32_t max_cs = static_cast<uint32_t>(base * constants::ZOOM_MAX);

    std::vector<uint32_t> levels = {
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
        12, 14, 15, 16, 18, 20, 24, 25, 28, 30, 32, 36, 40,
        45, 50, 60, 70, 75, 80, 90, 100, 120, 140, 150, 160, 180, 200,
        250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000
    };

    levels.erase(std::remove_if(levels.begin(), levels.end(),
        [min_cs, max_cs](uint32_t s) { return s < min_cs || s > max_cs; }),
        levels.end());

    return levels;
}

std::pair<float, float> RenderApp::screenToWorld(double sx, double sy, uint32_t cell_size) const {
    float visible_x = static_cast<float>(window_width_) / cell_size;
    float visible_y = static_cast<float>(window_height_) / cell_size;
    float frac_x = static_cast<float>(sx) / window_width_;
    float frac_y = static_cast<float>(sy) / window_height_;
    float world_x = viewport_.offset_x + frac_x * visible_x;
    float world_y = viewport_.offset_y + frac_y * visible_y;
    return {world_x, world_y};
}

void RenderApp::applyPan(double current_x, double current_y) {
    double delta_x = current_x - drag_state_.start_x;
    double delta_y = current_y - drag_state_.start_y;

    float visible_x = static_cast<float>(window_width_) / current_cell_size_;
    float visible_y = static_cast<float>(window_height_) / current_cell_size_;

    viewport_.offset_x = drag_state_.viewport_at_start.offset_x
        - static_cast<float>(delta_x) / window_width_ * visible_x;
    viewport_.offset_y = drag_state_.viewport_at_start.offset_y
        - static_cast<float>(delta_y) / window_height_ * visible_y;
    viewport_.offset_y = std::max(viewport_.offset_y, 0.0f);

    markViewportChanged();
}

void RenderApp::applyZoomAtPoint(uint32_t new_cell_size, double anchor_x, double anchor_y) {
    auto [world_x, world_y] = screenToWorld(anchor_x, anchor_y, current_cell_size_);
    float frac_x = static_cast<float>(anchor_x) / window_width_;
    float frac_y = static_cast<float>(anchor_y) / window_height_;

    current_cell_size_ = new_cell_size;

    float new_visible_x = static_cast<float>(window_width_) / new_cell_size;
    float new_visible_y = static_cast<float>(window_height_) / new_cell_size;

    viewport_.offset_x = world_x - frac_x * new_visible_x;
    viewport_.offset_y = world_y - frac_y * new_visible_y;
    viewport_.offset_y = std::max(viewport_.offset_y, 0.0f);

    markViewportChanged();
}

void RenderApp::handleZoom(float delta, double cursor_x, double cursor_y) {
    auto levels = generateZoomLevels();
    if (levels.empty()) return;

    uint32_t old_cell_size = current_cell_size_;

    // Find current index
    size_t current_idx = 0;
    for (size_t i = 0; i < levels.size(); ++i) {
        if (levels[i] >= old_cell_size) { current_idx = i; break; }
        if (i == levels.size() - 1) current_idx = i;
    }

    // Move to next/previous
    size_t new_idx;
    if (delta > 0.0f) {
        new_idx = std::min(current_idx + 1, levels.size() - 1);
    } else {
        new_idx = (current_idx > 0) ? current_idx - 1 : 0;
    }

    uint32_t new_cell_size = levels[new_idx];

    if (new_cell_size != old_cell_size) {
        applyZoomAtPoint(new_cell_size, cursor_x, cursor_y);
    }
}

// --- Input handlers ---

void RenderApp::handleResize(int width, int height) {
    uint32_t old_width = window_width_;
    uint32_t old_height = window_height_;

    window_width_ = static_cast<uint32_t>(width);
    window_height_ = static_cast<uint32_t>(height);

    if (width == 0 || height == 0) return;

    glViewport(0, 0, width, height);

    // Adjust viewport anchoring during windowed resizes (skip for fullscreen transitions)
    if (!is_fullscreen_) {
        int new_x, new_y;
        glfwGetWindowPos(window_, &new_x, &new_y);

        float old_visible_x = static_cast<float>(old_width) / current_cell_size_;
        float new_visible_x = static_cast<float>(window_width_) / current_cell_size_;
        float old_visible_y = static_cast<float>(old_height) / current_cell_size_;
        float new_visible_y = static_cast<float>(window_height_) / current_cell_size_;

        if (dpi_changing_) {
            // DPI change: keep viewport offset unchanged. The window's physical
            // screen size stays the same — only the pixel count changes.
        } else {
            // Normal resize: anchor opposite edge to the one being dragged
            if (new_x != windowed_x_ && old_width != window_width_) {
                float old_right = viewport_.offset_x + old_visible_x;
                viewport_.offset_x = old_right - new_visible_x;
            }
            if (new_y != windowed_y_ && old_height != window_height_) {
                float old_bottom = viewport_.offset_y + old_visible_y;
                viewport_.offset_y = old_bottom - new_visible_y;
                viewport_.offset_y = std::max(viewport_.offset_y, 0.0f);
            }
        }

        windowed_x_ = new_x;
        windowed_y_ = new_y;
    }

    // Render immediately with existing buffer to prevent DWM stretch artifacts.
    // With swap interval 0, glfwSwapBuffers returns instantly so this is cheap.
    // The WM_TIMER handler does periodic recomputes to fill in new content.
    markViewportChanged();
    render();
}

void RenderApp::handleScroll(double /*xoffset*/, double yoffset) {
    handleZoom(static_cast<float>(yoffset), cursor_x_, cursor_y_);
}

void RenderApp::handleMouseButton(int button, int action, int /*mods*/) {
    // Ignore synthetic mouse events generated by Windows touch input
    if (touch_state_.touch1.has_value()) return;

    if (button == GLFW_MOUSE_BUTTON_LEFT) {
        if (action == GLFW_PRESS) {
            glfwSetCursor(window_, hand_cursor_);
            drag_state_.active = true;
            drag_state_.start_x = cursor_x_;
            drag_state_.start_y = cursor_y_;
            drag_state_.viewport_at_start = viewport_;
        } else if (action == GLFW_RELEASE) {
            glfwSetCursor(window_, nullptr); // Default cursor
            drag_state_.active = false;
        }
    }
}

void RenderApp::handleCursorPos(double xpos, double ypos) {
    cursor_x_ = xpos;
    cursor_y_ = ypos;

    // Ignore synthetic mouse moves generated by Windows touch input
    if (touch_state_.touch1.has_value()) return;

    if (drag_state_.active) {
        applyPan(xpos, ypos);
    }
}

void RenderApp::enterFullscreen() {
    is_fullscreen_ = true;

#ifdef _WIN32
    HWND hwnd = glfwGetWin32Window(window_);

    saved_style_ = GetWindowLongW(hwnd, GWL_STYLE);

    // Save outer window rect in screen coordinates (works across monitors)
    RECT rect;
    GetWindowRect(hwnd, &rect);
    windowed_x_ = rect.left;
    windowed_y_ = rect.top;
    windowed_width_ = rect.right - rect.left;
    windowed_height_ = rect.bottom - rect.top;

    // Step 1: Apply style change (no move/resize yet)
    SetWindowLongW(hwnd, GWL_STYLE, saved_style_ & ~WS_OVERLAPPEDWINDOW);
    SetWindowPos(hwnd, nullptr, 0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED);

    // Step 2: Tell the shell we're fullscreen (hides taskbar, informs DWM)
    taskbar_mark_fullscreen(hwnd, true);

    // Step 3: Cover the current monitor.
    // Use height - 1 to prevent the GPU driver's "direct flip" optimization
    // from engaging. When an undecorated OpenGL window exactly matches a
    // monitor's dimensions, the driver bypasses DWM composition; on restore
    // DWM may not regain control properly, leaving a dark overlay. The 1px
    // bottom row of desktop is invisible in practice.
    HMONITOR hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    MONITORINFO mi = { sizeof(mi) };
    GetMonitorInfoW(hmon, &mi);
    SetWindowPos(hwnd, nullptr,
        mi.rcMonitor.left, mi.rcMonitor.top,
        mi.rcMonitor.right - mi.rcMonitor.left,
        (mi.rcMonitor.bottom - mi.rcMonitor.top) - 1,
        SWP_NOZORDER | SWP_ASYNCWINDOWPOS);

    InvalidateRgn(hwnd, nullptr, FALSE);
#else
    glfwGetWindowPos(window_, &windowed_x_, &windowed_y_);
    glfwGetWindowSize(window_, &windowed_width_, &windowed_height_);
    GLFWmonitor* monitor = get_current_monitor(window_);
    const GLFWvidmode* mode = glfwGetVideoMode(monitor);
    glfwSetWindowMonitor(window_, monitor, 0, 0,
        mode->width, mode->height, mode->refreshRate);
#endif
}

void RenderApp::exitFullscreen() {
    // Flush OpenGL before changing the window — ensures the driver releases
    // any display surfaces tied to the fullscreen monitor.
    glFinish();
    glfwSwapBuffers(window_);

#ifdef _WIN32
    HWND hwnd = glfwGetWin32Window(window_);

    // Hide the window first. This forces DWM to tear down the compositor
    // surface on the fullscreen monitor before we reposition. Without this,
    // a stale dark overlay can persist on secondary monitors.
    ShowWindow(hwnd, SW_HIDE);

    // Restore style (no move/resize yet)
    SetWindowLongW(hwnd, GWL_STYLE, saved_style_);
    SetWindowPos(hwnd, nullptr, 0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);

    taskbar_mark_fullscreen(hwnd, false);

    // Restore window at saved screen coordinates
    SetWindowPos(hwnd, nullptr,
        windowed_x_, windowed_y_, windowed_width_, windowed_height_,
        SWP_NOZORDER | SWP_NOOWNERZORDER);

    ShowWindow(hwnd, SW_SHOW);
    InvalidateRgn(hwnd, nullptr, FALSE);
#else
    glfwSetWindowMonitor(window_, nullptr,
        windowed_x_, windowed_y_, windowed_width_, windowed_height_, 0);
#endif
    is_fullscreen_ = false;

    // Update windowed position for edge-detection in handleResize
    glfwGetWindowPos(window_, &windowed_x_, &windowed_y_);
    glfwGetWindowSize(window_, &windowed_width_, &windowed_height_);
}

void RenderApp::handleKey(int key, int /*scancode*/, int action, int /*mods*/) {
    if (action != GLFW_PRESS) return;

    switch (key) {
    case GLFW_KEY_F11: {
        if (is_fullscreen_) {
            exitFullscreen();
        } else {
            enterFullscreen();
        }
        break;
    }
    case GLFW_KEY_ESCAPE:
        if (is_fullscreen_) {
            exitFullscreen();
        } else {
            std::cout << "Escape pressed, exiting..." << std::endl;
            glfwSetWindowShouldClose(window_, GLFW_TRUE);
        }
        break;
    case GLFW_KEY_0:
    case GLFW_KEY_KP_0:
        resetViewport();
        break;
    default:
        break;
    }
}

// --- Touch input ---

void RenderApp::handleTouchStart(uint64_t id, double x, double y) {
    // First touch
    if (!touch_state_.touch1.has_value()) {
        touch_state_.touch1 = TouchPoint{id, x, y};
        // Start single-touch pan
        touch_state_.single_touch = TouchPoint{id, x, y};
        drag_state_.active = true;
        drag_state_.start_x = x;
        drag_state_.start_y = y;
        drag_state_.viewport_at_start = viewport_;
    }
    // Second touch - start pinch zoom
    else if (!touch_state_.touch2.has_value()) {
        touch_state_.touch2 = TouchPoint{id, x, y};
        // Cancel single touch pan
        touch_state_.single_touch.reset();
        drag_state_.active = false;

        // Calculate initial distance for zoom ratio
        double dx = touch_state_.touch2->x - touch_state_.touch1->x;
        double dy = touch_state_.touch2->y - touch_state_.touch1->y;
        touch_state_.initial_distance = static_cast<float>(std::sqrt(dx * dx + dy * dy));
        touch_state_.initial_cell_size = current_cell_size_;
    }
}

void RenderApp::handleTouchMove(uint64_t id, double x, double y) {
    // Single touch pan
    if (touch_state_.single_touch.has_value() && touch_state_.single_touch->id == id) {
        if (drag_state_.active) {
            applyPan(x, y);
        }
    }
    // Pinch zoom
    else if (touch_state_.touch1.has_value() && touch_state_.touch2.has_value()) {
        // Update touch positions
        if (touch_state_.touch1->id == id) {
            touch_state_.touch1->x = x;
            touch_state_.touch1->y = y;
        }
        if (touch_state_.touch2->id == id) {
            touch_state_.touch2->x = x;
            touch_state_.touch2->y = y;
        }

        double dx = touch_state_.touch2->x - touch_state_.touch1->x;
        double dy = touch_state_.touch2->y - touch_state_.touch1->y;
        float current_distance = static_cast<float>(std::sqrt(dx * dx + dy * dy));

        if (touch_state_.initial_distance.has_value() &&
            touch_state_.initial_cell_size.has_value()) {

            float initial_distance = touch_state_.initial_distance.value();
            uint32_t initial_cell_size = touch_state_.initial_cell_size.value();

            if (initial_distance > 0.0f) {
                float zoom_factor = current_distance / initial_distance;
                float new_cs_f = static_cast<float>(initial_cell_size) * zoom_factor;
                new_cs_f = std::max(1.0f, std::min(new_cs_f, 500.0f));
                uint32_t target_cell_size = static_cast<uint32_t>(new_cs_f);

                // Find nearest zoom level
                auto levels = generateZoomLevels();
                uint32_t new_cell_size = target_cell_size;
                if (!levels.empty()) {
                    int32_t best_diff = INT32_MAX;
                    for (uint32_t level : levels) {
                        int32_t diff = std::abs(static_cast<int32_t>(level) - static_cast<int32_t>(target_cell_size));
                        if (diff < best_diff) {
                            best_diff = diff;
                            new_cell_size = level;
                        }
                    }
                }

                if (new_cell_size != current_cell_size_) {
                    // Anchor zoom to the pinch center (midpoint of two fingers)
                    double center_x = (touch_state_.touch1->x + touch_state_.touch2->x) / 2.0;
                    double center_y = (touch_state_.touch1->y + touch_state_.touch2->y) / 2.0;
                    applyZoomAtPoint(new_cell_size, center_x, center_y);
                }
            }
        }
    }
}

void RenderApp::handleTouchEnd(uint64_t id) {
    // Remove the ended touch
    if (touch_state_.touch1.has_value() && touch_state_.touch1->id == id) {
        touch_state_.touch1 = touch_state_.touch2;
        touch_state_.touch2.reset();
    } else if (touch_state_.touch2.has_value() && touch_state_.touch2->id == id) {
        touch_state_.touch2.reset();
    }

    // Clear single touch if it ended
    if (touch_state_.single_touch.has_value() && touch_state_.single_touch->id == id) {
        touch_state_.single_touch.reset();
        drag_state_.active = false;
    }

    // Reset pinch state if no touches remain
    if (!touch_state_.touch1.has_value()) {
        touch_state_.initial_distance.reset();
        touch_state_.initial_cell_size.reset();
    }

    // If one touch remains after pinch, restart pan
    if (touch_state_.touch1.has_value() && !touch_state_.touch2.has_value()) {
        touch_state_.single_touch = touch_state_.touch1;
        drag_state_.active = true;
        drag_state_.start_x = touch_state_.touch1->x;
        drag_state_.start_y = touch_state_.touch1->y;
        drag_state_.viewport_at_start = viewport_;
    }
}

#ifdef _WIN32
// --- Win32 WM_TOUCH window subclass ---

static constexpr UINT_PTR RESIZE_TIMER_ID = 1;

LRESULT CALLBACK touch_wndproc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    // When moving between monitors with different DPIs, GLFW resizes the window
    // to maintain visual size. Set a flag so handleResize uses center-preserving
    // viewport adjustment instead of edge-detection (which would shift the view).
    if (msg == WM_DPICHANGED && s_touch_app) {
        UINT new_dpi = HIWORD(wParam);
        UINT old_dpi = s_touch_app->currentDpi();
        RECT* suggested = reinterpret_cast<RECT*>(lParam);

        // Scale the client area by the DPI ratio to maintain visual size
        RECT client;
        GetClientRect(hwnd, &client);
        int new_w = MulDiv(client.right, new_dpi, old_dpi);
        int new_h = MulDiv(client.bottom, new_dpi, old_dpi);

        // Build a window rect from the desired client size
        RECT wr = { 0, 0, new_w, new_h };
        DWORD style = static_cast<DWORD>(GetWindowLongW(hwnd, GWL_STYLE));
        DWORD exstyle = static_cast<DWORD>(GetWindowLongW(hwnd, GWL_EXSTYLE));
        AdjustWindowRectExForDpi(&wr, style, FALSE, exstyle, new_dpi);

        s_touch_app->setDpiChanging(true);
        SetWindowPos(hwnd, nullptr,
            suggested->left, suggested->top,
            wr.right - wr.left, wr.bottom - wr.top,
            SWP_NOZORDER | SWP_NOACTIVATE);
        s_touch_app->setDpiChanging(false);
        s_touch_app->setCurrentDpi(new_dpi);
        // Immediately recompute + render for the new pixel dimensions
        s_touch_app->computeCA();
        s_touch_app->render();
        return 0;
    }

    // During the modal resize/move loop, the main event loop is blocked.
    // Use a Win32 timer to recompute + render so content reveals progressively
    // during resize, matching the Rust/winit version.
    if (msg == WM_ENTERSIZEMOVE) {
        SetTimer(hwnd, RESIZE_TIMER_ID, 100, nullptr);
        return 0;
    }
    if (msg == WM_EXITSIZEMOVE) {
        KillTimer(hwnd, RESIZE_TIMER_ID);
        if (s_touch_app) {
            s_touch_app->computeCA();
            s_touch_app->render();
        }
        return 0;
    }
    if (msg == WM_TIMER && wParam == RESIZE_TIMER_ID && s_touch_app) {
        // Compute only — render happens on every WM_SIZE via handleResize
        s_touch_app->computeCA();
        return 0;
    }

    if (msg == WM_TOUCH && s_touch_app) {
        UINT input_count = LOWORD(wParam);
        if (input_count > 0) {
            std::vector<TOUCHINPUT> inputs(input_count);
            if (GetTouchInputInfo(reinterpret_cast<HTOUCHINPUT>(lParam),
                                   input_count, inputs.data(), sizeof(TOUCHINPUT))) {
                for (UINT i = 0; i < input_count; ++i) {
                    const auto& ti = inputs[i];
                    // TOUCHINPUT coordinates are in hundredths of a pixel (centi-pixels)
                    POINT pt;
                    pt.x = TOUCH_COORD_TO_PIXEL(ti.x);
                    pt.y = TOUCH_COORD_TO_PIXEL(ti.y);
                    ScreenToClient(hwnd, &pt);

                    double x = static_cast<double>(pt.x);
                    double y = static_cast<double>(pt.y);
                    uint64_t id = static_cast<uint64_t>(ti.dwID);

                    if (ti.dwFlags & TOUCHEVENTF_DOWN) {
                        s_touch_app->handleTouchStart(id, x, y);
                    } else if (ti.dwFlags & TOUCHEVENTF_MOVE) {
                        s_touch_app->handleTouchMove(id, x, y);
                    } else if (ti.dwFlags & TOUCHEVENTF_UP) {
                        s_touch_app->handleTouchEnd(id);
                    }
                }
                CloseTouchInputHandle(reinterpret_cast<HTOUCHINPUT>(lParam));
                return 0;
            }
        }
    }
    return CallWindowProcW(s_original_wndproc, hwnd, msg, wParam, lParam);
}
#endif

// --- Main loop ---

void RenderApp::run() {
    computeCA();
    render(); // Initial render before entering event loop

    while (!glfwWindowShouldClose(window_)) {
        // Use wait with timeout when debounce is active, otherwise block
        if (last_viewport_change_.has_value() && needs_recompute_) {
            auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - last_viewport_change_.value()).count();
            int64_t remaining = static_cast<int64_t>(config_.debounce_ms) - elapsed;
            if (remaining > 0) {
                glfwWaitEventsTimeout(remaining / 1000.0);
            } else {
                glfwWaitEventsTimeout(0.001); // Minimal wait, then recompute
            }
        } else {
            glfwWaitEvents(); // Block until event
        }

        checkDebounceAndRecompute();
        render();
    }
}

} // namespace cae
