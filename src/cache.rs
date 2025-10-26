use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// A tile represents a fixed-size cached region of CA computation
/// Grid-based: tile at (x, y) covers cells [x*256..(x+1)*256] and generations [y*256..(y+1)*256]
/// The tile's position is tracked by TileKey in the cache HashMap
#[derive(Debug)]
pub struct Tile {
    pub buffer: wgpu::Buffer,
    pub simulated_width: u32,  // Buffer width (includes padding)
    pub padding_left: u32,     // Padding on left side
}

/// Cache key uniquely identifies a tile by its grid position
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub rule: u8,
    pub initial_state_hash: u64,
    pub tile_x: i32,  // Horizontal tile coordinate
    pub tile_y: i32,  // Vertical tile coordinate (generation-based)
}

impl TileKey {
    pub fn new(
        rule: u8,
        initial_state: &Option<String>,
        tile_x: i32,
        tile_y: i32,
    ) -> Self {
        let initial_state_hash = Self::hash_initial_state(initial_state);

        TileKey {
            rule,
            initial_state_hash,
            tile_x,
            tile_y,
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

    /// Tile dimensions (tiles are tile_size × tile_size cells)
    pub tile_size: u32,

    /// Cached tiles indexed by key
    tiles: HashMap<TileKey, Tile>,

    /// LRU queue: front = most recently used, back = least recently used
    lru_queue: VecDeque<TileKey>,

    /// Statistics
    pub hits: u64,
    pub misses: u64,
}

impl TileCache {
    pub fn new(max_tiles: usize, tile_size: u32) -> Self {
        println!("Initializing TileCache with max_tiles={}, tile_size={}x{}",
            max_tiles, tile_size, tile_size);
        TileCache {
            max_tiles,
            tile_size,
            tiles: HashMap::new(),
            lru_queue: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get a tile from cache if it exists
    pub fn get(&mut self, key: &TileKey) -> Option<&Tile> {
        if self.tiles.contains_key(key) {
            self.touch(key);
            self.hits += 1;
            println!("Cache HIT: tile ({}, {}) (hits={}, misses={})",
                key.tile_x, key.tile_y, self.hits, self.misses);
            return self.tiles.get(key);
        }

        self.misses += 1;
        println!("Cache MISS: tile ({}, {}) (hits={}, misses={})",
            key.tile_x, key.tile_y, self.hits, self.misses);
        None
    }

    /// Insert a tile into the cache
    pub fn insert(&mut self, key: TileKey, tile: Tile) {
        println!("Cache INSERT: tile ({}, {}), buffer_size={}x{} (cache_size={}/{})",
            key.tile_x, key.tile_y,
            tile.simulated_width, self.tile_size,
            self.tiles.len(), self.max_tiles);

        // If key already exists, remove it from LRU queue
        if self.tiles.contains_key(&key) {
            self.lru_queue.retain(|k| k != &key);
        }

        // Evict if at capacity
        while self.tiles.len() >= self.max_tiles && !self.lru_queue.is_empty() {
            if let Some(evict_key) = self.lru_queue.pop_back() {
                self.tiles.remove(&evict_key);
                println!("Cache EVICT: tile ({}, {}) (cache_size={}/{})",
                    evict_key.tile_x, evict_key.tile_y,
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
}
