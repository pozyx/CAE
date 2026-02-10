#include "compute.h"
#include "cache.h"
#include "config.h"

#include <algorithm>
#include <iostream>
#include <vector>

#include <cuda_runtime.h>

namespace cae {
namespace {

// Initialize the first row of a CA buffer based on initial state or single-cell default.
// Places cell values into the row accounting for padding and horizontal offset.
std::vector<uint32_t> init_first_row(
    uint32_t simulated_width,
    uint32_t padding,
    int32_t horizontal_offset,
    const std::optional<std::string>& initial_state)
{
    std::vector<uint32_t> row(simulated_width, 0);
    if (initial_state.has_value()) {
        const auto& state_str = initial_state.value();
        int32_t base_offset = static_cast<int32_t>(padding) - horizontal_offset;
        for (size_t i = 0; i < state_str.size(); ++i) {
            int32_t pos = base_offset + static_cast<int32_t>(i);
            if (pos >= 0 && static_cast<uint32_t>(pos) < simulated_width) {
                row[pos] = (state_str[i] == '1') ? 1 : 0;
            }
        }
    } else {
        int32_t world_zero = static_cast<int32_t>(padding) - horizontal_offset;
        if (world_zero >= 0 && static_cast<uint32_t>(world_zero) < simulated_width) {
            row[world_zero] = 1;
        }
    }
    return row;
}

// Allocate a CUDA buffer, copy initial row, and run CA kernel for the specified number of generations.
// Returns a device pointer to the full computed buffer (caller must cudaFree).
uint32_t* compute_ca_buffer(
    const std::vector<uint32_t>& initial_row,
    uint32_t simulated_width,
    uint32_t buffer_height,
    uint32_t total_generations,
    uint8_t rule)
{
    uint64_t total_cells = static_cast<uint64_t>(simulated_width) * buffer_height;
    size_t buffer_size = total_cells * sizeof(uint32_t);

    uint32_t* d_ca_state = nullptr;
    cudaMalloc(&d_ca_state, buffer_size);
    cudaMemset(d_ca_state, 0, buffer_size);
    // Upload only the initial row (small) — rest is already zeroed on GPU
    cudaMemcpy(d_ca_state, initial_row.data(),
               initial_row.size() * sizeof(uint32_t), cudaMemcpyHostToDevice);

    uint32_t batch_size = constants::COMPUTE_BATCH_SIZE;
    for (uint32_t batch_start = 0; batch_start < total_generations; batch_start += batch_size) {
        uint32_t batch_end = std::min(batch_start + batch_size, total_generations);
        for (uint32_t iter = batch_start; iter < batch_end; ++iter) {
            launch_ca_step(d_ca_state, simulated_width, buffer_height, rule, iter, nullptr);
        }
        cudaDeviceSynchronize();
    }

    return d_ca_state;
}

} // anonymous namespace

void free_ca_result(CaResult& result) {
    if (result.d_buffer) {
        cudaFree(result.d_buffer);
        result.d_buffer = nullptr;
    }
    result.buffer_size_bytes = 0;
}

CaResult run_ca(
    uint8_t rule,
    uint32_t start_generation,
    uint32_t iterations,
    uint32_t visible_width,
    int32_t horizontal_offset,
    const std::optional<std::string>& initial_state)
{
    // Add padding for boundary simulation
    uint32_t total_generations = start_generation + iterations;
    uint32_t padding = total_generations;
    uint32_t simulated_width = visible_width + 2 * padding;

    std::cout << "Visible width: " << visible_width
              << ", Simulated width: " << simulated_width
              << " (padding: " << padding << ")" << std::endl;
    std::cout << "Computing generations " << start_generation
              << " to " << total_generations
              << ", horizontal offset: " << horizontal_offset << std::endl;

    uint32_t buffer_height = total_generations + 1;

    auto initial_row = init_first_row(simulated_width, padding, horizontal_offset, initial_state);
    uint32_t* d_ca_state = compute_ca_buffer(initial_row, simulated_width, buffer_height, total_generations, rule);

    // Extract output buffer containing only the visible range
    uint32_t visible_height = iterations + 1;
    size_t visible_buffer_size = static_cast<size_t>(simulated_width) * visible_height * sizeof(uint32_t);

    uint32_t* d_output = nullptr;
    cudaMalloc(&d_output, visible_buffer_size);

    cudaMemcpy(d_output, d_ca_state + start_generation * simulated_width,
               visible_buffer_size, cudaMemcpyDeviceToDevice);

    // Free the full computation buffer
    cudaFree(d_ca_state);

    CaResult result;
    result.d_buffer = d_output;
    result.simulated_width = simulated_width;
    result.visible_width = visible_width;
    result.height = visible_height;
    result.padding_left = padding;
    result.buffer_size_bytes = visible_buffer_size;
    return result;
}

CaResult run_ca_with_cache(
    uint8_t rule,
    uint32_t start_generation,
    uint32_t iterations,
    uint32_t visible_width,
    int32_t horizontal_offset,
    const std::optional<std::string>& initial_state,
    TileCache& cache)
{
    std::cout << "\n=== run_ca_with_cache: gen " << start_generation
              << ".." << start_generation + iterations
              << ", offset_x=" << horizontal_offset
              << ", width=" << visible_width << " ===" << std::endl;

    int32_t viewport_x_start = horizontal_offset;
    int32_t viewport_x_end = horizontal_offset + static_cast<int32_t>(visible_width);
    int32_t viewport_y_start = static_cast<int32_t>(start_generation);
    int32_t viewport_y_end = static_cast<int32_t>(start_generation + iterations);

    int32_t tile_size = static_cast<int32_t>(cache.tile_size);

    // Euclidean division for tile coordinates
    auto div_euclid = [](int32_t a, int32_t b) -> int32_t {
        int32_t d = a / b;
        if (a % b != 0 && (a ^ b) < 0) d -= 1;
        return d;
    };

    int32_t tile_x_start = div_euclid(viewport_x_start, tile_size);
    int32_t tile_x_end = div_euclid(viewport_x_end - 1, tile_size);
    int32_t tile_y_start = div_euclid(viewport_y_start, tile_size);
    int32_t tile_y_end = div_euclid(viewport_y_end - 1, tile_size);

    std::cout << "Viewport needs tiles: X=" << tile_x_start << ".." << tile_x_end
              << ", Y=" << tile_y_start << ".." << tile_y_end << std::endl;

    // Compute missing tiles
    for (int32_t ty = tile_y_start; ty <= tile_y_end; ++ty) {
        for (int32_t tx = tile_x_start; tx <= tile_x_end; ++tx) {
            auto tile_key = TileKey::create(rule, initial_state, tx, ty);
            if (!cache.get(tile_key)) {
                std::cout << "Computing new tile (" << tx << ", " << ty << ")" << std::endl;
                uint32_t tw = static_cast<uint32_t>(tile_size);
                int32_t tile_start_x = tx * tile_size;
                int32_t tile_end_x = tile_start_x + tile_size;
                int32_t generation_start = ty * tile_size;
                int32_t generation_end = (ty + 1) * tile_size;
                std::cout << "Computing tile (" << tx << ", " << ty
                          << "): cells " << tile_start_x << ".." << tile_end_x
                          << ", generations " << generation_start << ".." << generation_end
                          << std::endl;
                uint32_t tile_padding = static_cast<uint32_t>(std::max(generation_end, 0));
                uint32_t sim_width = tw + 2 * tile_padding;
                uint32_t total_gens = static_cast<uint32_t>(std::max(generation_end, 0));
                uint32_t buf_height = total_gens + 1;

                auto initial_row = init_first_row(sim_width, tile_padding, tile_start_x, initial_state);
                uint32_t* d_ca = compute_ca_buffer(initial_row, sim_width, buf_height, total_gens, rule);

                // Extract tile generation range
                int32_t tile_gen_start = ty * tile_size;
                uint32_t tile_gen_offset = static_cast<uint32_t>(std::max(tile_gen_start, 0));
                size_t tile_buf_size = static_cast<size_t>(sim_width) * tw * sizeof(uint32_t);

                uint32_t* d_tile = nullptr;
                cudaMalloc(&d_tile, tile_buf_size);
                cudaMemcpy(d_tile, d_ca + tile_gen_offset * sim_width,
                           tile_buf_size, cudaMemcpyDeviceToDevice);
                cudaFree(d_ca);

                Tile new_tile;
                new_tile.d_buffer = d_tile;
                new_tile.simulated_width = sim_width;
                new_tile.padding_left = tile_padding;
                new_tile.buffer_size_bytes = tile_buf_size;

                cache.insert(tile_key, std::move(new_tile));
            } else {
                std::cout << "Using cached tile (" << tx << ", " << ty << ")" << std::endl;
            }
        }
    }

    // Assemble output buffer
    uint32_t total_generations = start_generation + iterations;
    uint32_t padding = total_generations;
    uint32_t simulated_width = visible_width + 2 * padding;
    uint32_t output_height = iterations + 1;
    size_t output_buf_size = static_cast<size_t>(simulated_width) * output_height * sizeof(uint32_t);

    std::cout << "Output buffer: width=" << simulated_width
              << ", height=" << output_height
              << ", padding=" << padding << std::endl;

    uint32_t* d_output = nullptr;
    cudaMalloc(&d_output, output_buf_size);
    cudaMemset(d_output, 0, output_buf_size);

    // Copy relevant regions from tiles to output buffer
    for (int32_t ty = tile_y_start; ty <= tile_y_end; ++ty) {
        for (int32_t tx = tile_x_start; tx <= tile_x_end; ++tx) {
            auto tile_key = TileKey::create(rule, initial_state, tx, ty);
            Tile* tile = cache.get(tile_key);
            if (!tile) continue;

            int32_t tile_world_x_start = tx * tile_size;
            int32_t tile_world_x_end = tile_world_x_start + tile_size;
            int32_t tile_gen_start = ty * tile_size;
            int32_t tile_gen_end = tile_gen_start + tile_size;

            int32_t copy_x_start = std::max(viewport_x_start, tile_world_x_start);
            int32_t copy_x_end = std::min(viewport_x_end, tile_world_x_end);
            int32_t copy_gen_start = std::max(viewport_y_start, tile_gen_start);
            int32_t copy_gen_end = std::min(viewport_y_end, tile_gen_end);

            if (copy_x_end <= copy_x_start || copy_gen_end <= copy_gen_start) continue;

            for (int32_t gen = copy_gen_start; gen < copy_gen_end; ++gen) {
                uint32_t gen_in_viewport = static_cast<uint32_t>(gen - viewport_y_start);
                uint32_t gen_in_tile = static_cast<uint32_t>(gen - tile_gen_start);
                uint32_t slice_width = static_cast<uint32_t>(copy_x_end - copy_x_start);

                uint32_t x_in_tile_buffer = static_cast<uint32_t>(copy_x_start - tile_world_x_start) + tile->padding_left;
                uint32_t x_in_output_buffer = static_cast<uint32_t>(copy_x_start - viewport_x_start) + padding;

                if (gen_in_tile >= static_cast<uint32_t>(tile_size) || gen_in_viewport >= iterations) continue;
                if (x_in_tile_buffer + slice_width > tile->simulated_width) continue;
                if (x_in_output_buffer + slice_width > simulated_width) continue;

                size_t src_offset = (static_cast<size_t>(gen_in_tile) * tile->simulated_width + x_in_tile_buffer);
                size_t dst_offset = (static_cast<size_t>(gen_in_viewport) * simulated_width + x_in_output_buffer);

                cudaMemcpy(d_output + dst_offset, tile->d_buffer + src_offset,
                           slice_width * sizeof(uint32_t), cudaMemcpyDeviceToDevice);
            }
        }
    }

    cudaDeviceSynchronize();

    CaResult result;
    result.d_buffer = d_output;
    result.simulated_width = simulated_width;
    result.visible_width = visible_width;
    result.height = output_height;
    result.padding_left = padding;
    result.buffer_size_bytes = output_buf_size;
    return result;
}

} // namespace cae
