#pragma once
#include <chrono>
#include <cstdint>
#include <optional>
#include <vector>

#include <glad/glad.h>
#include <GLFW/glfw3.h>

#include "cache.h"
#include "compute.h"
#include "config.h"
#include "viewport.h"

struct cudaGraphicsResource;

namespace cae {

// Matches the RenderParams uniform in the GLSL fragment shader
struct RenderParams {
    uint32_t visible_width;
    uint32_t visible_height;
    uint32_t simulated_width;
    uint32_t padding_left;
    uint32_t cell_size;
    uint32_t window_width;
    uint32_t window_height;
    int32_t viewport_offset_x;
    int32_t viewport_offset_y;
    int32_t buffer_offset_x;
    int32_t buffer_offset_y;
    uint32_t _padding;
};

class RenderApp {
public:
    explicit RenderApp(const Config& config);
    ~RenderApp();
    void run(); // Main loop (blocks until exit)

    // GLFW callback dispatchers (must be public for static callbacks)
    void handleResize(int width, int height);
    void handleScroll(double xoffset, double yoffset);
    void handleMouseButton(int button, int action, int mods);
    void handleCursorPos(double xpos, double ypos);
    void handleKey(int key, int scancode, int action, int mods);
    void handleTouchStart(uint64_t id, double x, double y);
    void handleTouchMove(uint64_t id, double x, double y);
    void handleTouchEnd(uint64_t id);

    // Public for WM_TIMER resize recompute from wndproc
    void computeCA();
    void render();
    void checkDebounceAndRecompute();
    void setDpiChanging(bool v) { dpi_changing_ = v; }
    unsigned int currentDpi() const { return current_dpi_; }
    void setCurrentDpi(unsigned int dpi) { current_dpi_ = dpi; }

private:
    // Initialization
    void initGLFW();
    void initOpenGL();
    void initShaders();
    void initFullScreenQuad();

    // Core operations
    void updateRenderParams();
    void markViewportChanged();
    void resetViewport();

    // Pan / zoom helpers
    void applyPan(double current_x, double current_y);
    void applyZoomAtPoint(uint32_t new_cell_size, double anchor_x, double anchor_y);
    void handleZoom(float delta, double cursor_x, double cursor_y);
    std::vector<uint32_t> generateZoomLevels() const;
    std::pair<float, float> screenToWorld(double sx, double sy, uint32_t cell_size) const;

    // Fullscreen toggle (Win32 borderless on Windows, GLFW exclusive elsewhere)
    void enterFullscreen();
    void exitFullscreen();

    // State
    Config config_;
    GLFWwindow* window_ = nullptr;
    GLFWcursor* hand_cursor_ = nullptr;

    // OpenGL resources
    GLuint shader_program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLuint ebo_ = 0;
    GLuint ca_ssbo_ = 0;       // SSBO for CA data
    GLuint params_ubo_ = 0;    // Uniform buffer for render params
    size_t ca_ssbo_size_ = 0;  // Current SSBO size

    // CUDA-GL interop: maps SSBO for direct GPU writes (zero-copy)
    cudaGraphicsResource* cuda_ssbo_resource_ = nullptr;

    // Viewport
    Viewport viewport_;
    Viewport buffer_viewport_;
    DragState drag_state_;
    TouchState touch_state_;
    bool needs_recompute_ = true;
    double cursor_x_ = 0.0;
    double cursor_y_ = 0.0;

    // Window/cell dimensions
    uint32_t window_width_;
    uint32_t window_height_;
    uint32_t current_cell_size_;

    // Buffer metadata (from last compute)
    uint32_t buffer_simulated_width_ = 0;
    uint32_t buffer_visible_width_ = 0;
    uint32_t buffer_visible_height_ = 0;
    uint32_t buffer_padding_left_ = 0;

    // Cache
    std::optional<TileCache> cache_;

    // Timing
    std::optional<std::chrono::steady_clock::time_point> last_viewport_change_;
    std::optional<std::chrono::steady_clock::time_point> last_params_update_;

    // DPI / fullscreen state
    bool dpi_changing_ = false;
    unsigned int current_dpi_ = 96;
    bool is_fullscreen_ = false;
    int windowed_x_ = 0, windowed_y_ = 0;
    int windowed_width_ = 0, windowed_height_ = 0;
#ifdef _WIN32
    unsigned long saved_style_ = 0;
#endif
};

} // namespace cae
