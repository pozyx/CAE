use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// A tile represents a cached region of CA computation
/// For Option A: tiles are generation ranges across the full horizontal viewport
#[derive(Debug)]
pub struct Tile {
    pub buffer: wgpu::Buffer,
    pub generation_start: u32,
    pub generation_end: u32,      // Exclusive
    pub horizontal_start: i32,    // World-space cell coordinate
    pub horizontal_end: i32,      // Exclusive, world-space
    pub simulated_width: u32,     // Buffer width (includes padding)
    pub padding_left: u32,        // Padding on left side
}

/// Cache key uniquely identifies a tile
/// For Option A: Only generation range matters, horizontal range is stored but not used for matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub rule: u8,
    pub initial_state_hash: u64,
    pub generation_start: u32,
    pub generation_end: u32,
    // Note: horizontal range is NOT part of the cache key for Option A
    // We store it in the Tile for reference, but tiles are matched only by generation range
}

impl TileKey {
    pub fn new(
        rule: u8,
        initial_state: &Option<String>,
        generation_start: u32,
        generation_end: u32,
        _horizontal_start: i32,  // Ignored for Option A
        _horizontal_end: i32,    // Ignored for Option A
    ) -> Self {
        let initial_state_hash = Self::hash_initial_state(initial_state);

        // For Option A: Cache key only includes generation range
        // Horizontal range is ignored - we cache full horizontal spans
        TileKey {
            rule,
            initial_state_hash,
            generation_start,
            generation_end,
        }
    }

    fn hash_initial_state(initial_state: &Option<String>) -> u64 {
        let mut hasher = DefaultHasher::new();
        initial_state.hash(&mut hasher);
        hasher.finish()
    }
}

/// LRU tile cache for CA computation results
pub struct TileCache {
    /// Maximum number of tiles to cache
    max_tiles: usize,

    /// Cached tiles indexed by key
    tiles: HashMap<TileKey, Tile>,

    /// LRU queue: front = most recently used, back = least recently used
    lru_queue: VecDeque<TileKey>,

    /// Statistics
    pub hits: u64,
    pub misses: u64,
}

impl TileCache {
    pub fn new(max_tiles: usize) -> Self {
        println!("Initializing TileCache with max_tiles={}", max_tiles);
        TileCache {
            max_tiles,
            tiles: HashMap::new(),
            lru_queue: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get a tile from cache if it exists
    pub fn get(&mut self, key: &TileKey) -> Option<&Tile> {
        if self.tiles.contains_key(key) {
            // Move to front of LRU queue (most recently used)
            self.touch(key);
            self.hits += 1;
            println!("Cache HIT: gen {}..{} (hits={}, misses={})",
                key.generation_start, key.generation_end,
                self.hits, self.misses);
            self.tiles.get(key)
        } else {
            self.misses += 1;
            println!("Cache MISS: gen {}..{} (hits={}, misses={})",
                key.generation_start, key.generation_end,
                self.hits, self.misses);
            None
        }
    }

    /// Insert a tile into the cache
    pub fn insert(&mut self, key: TileKey, tile: Tile) {
        println!("Cache INSERT: gen {}..{}, horiz {}..{}, buffer_size={}x{} (cache_size={}/{})",
            key.generation_start, key.generation_end,
            tile.horizontal_start, tile.horizontal_end,
            tile.simulated_width, key.generation_end - key.generation_start,
            self.tiles.len(), self.max_tiles);

        // If key already exists, remove it from LRU queue
        if self.tiles.contains_key(&key) {
            self.lru_queue.retain(|k| k != &key);
        }

        // Evict if at capacity
        while self.tiles.len() >= self.max_tiles && !self.lru_queue.is_empty() {
            if let Some(evict_key) = self.lru_queue.pop_back() {
                self.tiles.remove(&evict_key);
                println!("Cache EVICT: gen {}..{} (cache_size={}/{})",
                    evict_key.generation_start, evict_key.generation_end,
                    self.tiles.len(), self.max_tiles);
            }
        }

        // Insert new tile
        self.tiles.insert(key.clone(), tile);
        self.lru_queue.push_front(key);
    }

    /// Mark a key as recently used (move to front of LRU)
    fn touch(&mut self, key: &TileKey) {
        self.lru_queue.retain(|k| k != key);
        self.lru_queue.push_front(key.clone());
    }

    /// Clear all cached tiles
    pub fn clear(&mut self) {
        println!("Cache CLEAR: removing {} tiles", self.tiles.len());
        self.tiles.clear();
        self.lru_queue.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.tiles.len(),
            max_size: self.max_tiles,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {}/{} tiles, {} hits, {} misses, {:.1}% hit rate",
            self.size,
            self.max_size,
            self.hits,
            self.misses,
            self.hit_rate * 100.0
        )
    }
}
