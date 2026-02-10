cmake_minimum_required(VERSION 3.18)

# Allow CUDA to work with newer MSVC versions (e.g. VS 2026)
set(CMAKE_CUDA_FLAGS "${CMAKE_CUDA_FLAGS} -allow-unsupported-compiler")

project(CAE-CUDA LANGUAGES C CXX CUDA)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CUDA_STANDARD 17)
set(CMAKE_CUDA_STANDARD_REQUIRED ON)

# Fetch GLFW
include(FetchContent)
FetchContent_Declare(
    glfw
    GIT_REPOSITORY https://github.com/glfw/glfw.git
    GIT_TAG        3.4
)
set(GLFW_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(glfw)

# Fetch CLI11
FetchContent_Declare(
    cli11
    GIT_REPOSITORY https://github.com/CLIUtils/CLI11.git
    GIT_TAG        v2.4.1
)
FetchContent_MakeAvailable(cli11)

# Find CUDA Toolkit
find_package(CUDAToolkit REQUIRED)

# Find OpenGL
find_package(OpenGL REQUIRED)

# GLAD (pre-generated, included in project)
add_library(glad STATIC external/glad/src/glad.c)
target_include_directories(glad PUBLIC external/glad/include)

# Main executable
add_executable(CAE-CUDA
    src/main.cpp
    src/config.cpp
    src/compute.cpp
    src/compute_kernel.cu
    src/cache.cpp
    src/render.cpp
)

target_include_directories(CAE-CUDA PRIVATE src)
target_link_libraries(CAE-CUDA PRIVATE
    glfw
    glad
    OpenGL::GL
    CUDA::cudart
    CLI11::CLI11
)

# Win32 APIs for DWM dark mode, fullscreen taskbar, DPI, and touch input
if(WIN32)
    target_link_libraries(CAE-CUDA PRIVATE dwmapi ole32)
endif()

# CUDA architectures (CUDA 13.1 supports sm_75+)
set_target_properties(CAE-CUDA PROPERTIES
    CUDA_ARCHITECTURES "75;80;86;89;90"
)

# Copy shaders to build directory
add_custom_command(TARGET CAE-CUDA POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
    ${CMAKE_SOURCE_DIR}/shaders
    $<TARGET_FILE_DIR:CAE-CUDA>/shaders
)
