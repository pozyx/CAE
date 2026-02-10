#pragma once
#include <cstdint>
#include <deque>
#include <functional>
#include <optional>
#include <string>
#include <unordered_map>

namespace cae {

struct Tile {
    uint32_t* d_buffer = nullptr;   // Device pointer (CUDA memory)
    uint32_t simulated_width = 0;
    uint32_t padding_left = 0;
    size_t buffer_size_bytes = 0;

    Tile() = default;
    ~Tile();
    Tile(Tile&& other) noexcept;
    Tile& operator=(Tile&& other) noexcept;
    Tile(const Tile&) = delete;
    Tile& operator=(const Tile&) = delete;
};

struct TileKey {
    uint8_t rule;
    uint64_t initial_state_hash;
    int32_t tile_x;
    int32_t tile_y;

    bool operator==(const TileKey& other) const;

    static TileKey create(uint8_t rule,
                          const std::optional<std::string>& initial_state,
                          int32_t tile_x, int32_t tile_y);
};

struct TileKeyHash {
    size_t operator()(const TileKey& key) const;
};

class TileCache {
public:
    TileCache(size_t max_tiles, uint32_t tile_size);

    // Returns pointer to tile if cached, nullptr on miss
    Tile* get(const TileKey& key);

    // Insert tile into cache (moves tile in, evicts LRU if full)
    void insert(TileKey key, Tile tile);

    uint32_t tile_size;
    uint64_t hits = 0;
    uint64_t misses = 0;

private:
    size_t max_tiles_;
    std::unordered_map<TileKey, Tile, TileKeyHash> tiles_;
    std::deque<TileKey> lru_queue_;
    void touch(const TileKey& key);
};

} // namespace cae
