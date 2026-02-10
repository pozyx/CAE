#pragma once
#include <cstdint>
#include <optional>
#include <string>

namespace cae {

class TileCache; // Forward declaration

struct CaResult {
    uint32_t* d_buffer = nullptr;   // Device pointer (CUDA memory)
    uint32_t simulated_width = 0;
    uint32_t visible_width = 0;
    uint32_t height = 0;
    uint32_t padding_left = 0;
    size_t buffer_size_bytes = 0;
};

// Compute CA without caching (direct mode)
CaResult run_ca(
    uint8_t rule,
    uint32_t start_generation,
    uint32_t iterations,
    uint32_t visible_width,
    int32_t horizontal_offset,
    const std::optional<std::string>& initial_state
);

// Compute CA with tile-based caching
CaResult run_ca_with_cache(
    uint8_t rule,
    uint32_t start_generation,
    uint32_t iterations,
    uint32_t visible_width,
    int32_t horizontal_offset,
    const std::optional<std::string>& initial_state,
    TileCache& cache
);

// Free a CaResult's device buffer
void free_ca_result(CaResult& result);

} // namespace cae

// CUDA kernel launch wrapper (defined in compute_kernel.cu)
extern "C" void launch_ca_step(
    uint32_t* d_ca_state,
    uint32_t width,
    uint32_t height,
    uint32_t rule,
    uint32_t current_row,
    void* stream
);
