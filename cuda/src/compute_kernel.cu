#include <cstdint>
#include <cuda_runtime.h>
#include "config.h"

struct CaParams {
    uint32_t width;
    uint32_t height;
    uint32_t rule;
    uint32_t current_row;
};

__device__ uint32_t get_cell(const uint32_t* ca_state, int x, const CaParams& p) {
    if (x < 0 || x >= static_cast<int>(p.width)) return 0;
    return ca_state[p.current_row * p.width + x];
}

__device__ uint32_t apply_rule(uint32_t left, uint32_t center, uint32_t right, uint32_t rule) {
    uint32_t pattern = (left << 2u) | (center << 1u) | right;
    uint32_t bit_mask = 1u << pattern;
    return (rule & bit_mask) >> pattern;
}

__global__ void ca_step_kernel(uint32_t* ca_state, CaParams params) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    if (x >= static_cast<int>(params.width)) return;

    uint32_t left   = get_cell(ca_state, x - 1, params);
    uint32_t center = get_cell(ca_state, x,     params);
    uint32_t right  = get_cell(ca_state, x + 1, params);

    uint32_t next = apply_rule(left, center, right, params.rule);
    uint32_t next_row = params.current_row + 1;
    ca_state[next_row * params.width + x] = next;
}

extern "C" void launch_ca_step(
    uint32_t* d_ca_state,
    uint32_t width,
    uint32_t height,
    uint32_t rule,
    uint32_t current_row,
    void* stream)
{
    CaParams params = {width, height, rule, current_row};
    constexpr int blockSize = cae::constants::COMPUTE_BLOCK_SIZE;
    int gridSize = (width + blockSize - 1) / blockSize;
    ca_step_kernel<<<gridSize, blockSize, 0, static_cast<cudaStream_t>(stream)>>>(d_ca_state, params);
}
