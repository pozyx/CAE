#include "cache.h"

#include <algorithm>
#include <functional>
#include <iostream>

#include <cuda_runtime.h>

namespace cae {

// --- Tile ---

Tile::~Tile() {
    if (d_buffer) {
        cudaFree(d_buffer);
        d_buffer = nullptr;
    }
}

Tile::Tile(Tile&& other) noexcept
    : d_buffer(other.d_buffer)
    , simulated_width(other.simulated_width)
    , padding_left(other.padding_left)
    , buffer_size_bytes(other.buffer_size_bytes)
{
    other.d_buffer = nullptr;
    other.buffer_size_bytes = 0;
}

Tile& Tile::operator=(Tile&& other) noexcept {
    if (this != &other) {
        if (d_buffer) cudaFree(d_buffer);
        d_buffer = other.d_buffer;
        simulated_width = other.simulated_width;
        padding_left = other.padding_left;
        buffer_size_bytes = other.buffer_size_bytes;
        other.d_buffer = nullptr;
        other.buffer_size_bytes = 0;
    }
    return *this;
}

// --- TileKey ---

bool TileKey::operator==(const TileKey& other) const {
    return rule == other.rule
        && initial_state_hash == other.initial_state_hash
        && tile_x == other.tile_x
        && tile_y == other.tile_y;
}

TileKey TileKey::create(uint8_t rule,
                        const std::optional<std::string>& initial_state,
                        int32_t tile_x, int32_t tile_y)
{
    TileKey key;
    key.rule = rule;
    key.tile_x = tile_x;
    key.tile_y = tile_y;

    // Hash the initial state
    std::hash<std::string> hasher;
    if (initial_state.has_value()) {
        key.initial_state_hash = hasher(initial_state.value());
    } else {
        key.initial_state_hash = 0;
    }
    return key;
}

size_t TileKeyHash::operator()(const TileKey& key) const {
    size_t h = std::hash<uint8_t>{}(key.rule);
    h ^= std::hash<uint64_t>{}(key.initial_state_hash) + 0x9e3779b9 + (h << 6) + (h >> 2);
    h ^= std::hash<int32_t>{}(key.tile_x) + 0x9e3779b9 + (h << 6) + (h >> 2);
    h ^= std::hash<int32_t>{}(key.tile_y) + 0x9e3779b9 + (h << 6) + (h >> 2);
    return h;
}

// --- TileCache ---

TileCache::TileCache(size_t max_tiles, uint32_t tile_size)
    : tile_size(tile_size > 0 ? tile_size : 256)
    , max_tiles_(max_tiles)
{
    std::cout << "TileCache: " << max_tiles << " tiles, "
              << this->tile_size << "x" << this->tile_size
              << " cells (~" << (this->tile_size * this->tile_size * 4) / 1024
              << " KB/tile)" << std::endl;
}

Tile* TileCache::get(const TileKey& key) {
    auto it = tiles_.find(key);
    if (it != tiles_.end()) {
        touch(key);
        hits++;
        std::cout << "Cache HIT: tile (" << key.tile_x << ", " << key.tile_y
                  << ") (hits=" << hits << ", misses=" << misses << ")" << std::endl;
        return &it->second;
    }
    misses++;
    std::cout << "Cache MISS: tile (" << key.tile_x << ", " << key.tile_y
              << ") (hits=" << hits << ", misses=" << misses << ")" << std::endl;
    return nullptr;
}

void TileCache::insert(TileKey key, Tile tile) {
    std::cout << "Cache INSERT: tile (" << key.tile_x << ", " << key.tile_y
              << "), buffer_size=" << tile.simulated_width << "x" << tile_size
              << " (cache_size=" << tiles_.size() << "/" << max_tiles_ << ")" << std::endl;

    // If key already exists, remove from LRU
    if (tiles_.count(key)) {
        lru_queue_.erase(
            std::remove(lru_queue_.begin(), lru_queue_.end(), key),
            lru_queue_.end());
    }

    // Evict if at capacity
    while (tiles_.size() >= max_tiles_ && !lru_queue_.empty()) {
        auto evict_key = lru_queue_.back();
        lru_queue_.pop_back();
        tiles_.erase(evict_key);
        std::cout << "Cache EVICT: tile (" << evict_key.tile_x << ", " << evict_key.tile_y
                  << ") (cache_size=" << tiles_.size() << "/" << max_tiles_ << ")" << std::endl;
    }

    tiles_.emplace(key, std::move(tile));
    lru_queue_.push_front(key);
}

void TileCache::touch(const TileKey& key) {
    lru_queue_.erase(
        std::remove(lru_queue_.begin(), lru_queue_.end(), key),
        lru_queue_.end());
    lru_queue_.push_front(key);
}

} // namespace cae
