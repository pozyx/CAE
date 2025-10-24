# Caching System - Future Improvements Notes

## Current Implementation (v1.2.0)

The current tile-based cache system:
- Divides CA space into fixed 256×256 cell tiles
- Each tile identified by grid coordinates `(tile_x, tile_y)`
- **All tiles compute from generation 0**
- LRU eviction when cache is full
- Cache hit rate: 70-90% during normal navigation

### Performance Characteristics
- **First view at generation N**: Slow (computes N generations)
- **Subsequent pans/zooms at generation N**: Fast (tiles cached)
- **Scrolling to deeper generations**: Progressively slower (more generations to compute)

## Attempted: Incremental Tile Computation (Checkpointing)

### Goal
Avoid recomputing from generation 0 by building tiles incrementally from previous tiles.

### Approach Tried
```
Tile (x, 0): Compute gen 0-255 from initial state
Tile (x, 1): Copy last row of Tile (x, 0), compute gen 256-511
Tile (x, 2): Copy last row of Tile (x, 1), compute gen 512-767
...
```

### Fundamental Problem: Variable Padding Requirements

**Issue**: Each tile needs different padding based on generation depth.

- Tile at generation 0-255 needs padding = 256 cells
  - Buffer width = 256 + 2×256 = 768 cells
- Tile at generation 256-511 needs padding = 512 cells
  - Buffer width = 256 + 2×512 = 1280 cells
- Tile at generation 512-767 needs padding = 768 cells
  - Buffer width = 256 + 2×768 = 1792 cells

**Why padding grows**: Cellular automaton patterns can expand by 1 cell per generation in each direction. After N generations, pattern may have expanded N cells left and N cells right from original position.

**The incompatibility**:
1. When computing Tile (x, 1) from Tile (x, 0), we need to copy the last row
2. Tile (x, 0) has 768 cells of data (padding=256)
3. Tile (x, 1) needs 1280 cells (padding=512)
4. The 768 cells from Tile (x, 0) don't have enough context for the edges of Tile (x, 1)
5. Cells near the edges of the new tile lack the necessary neighbor information

**What we observed**: When testing, horizontal panning at later generations showed missing/corrupted tiles. The incremental computation worked vertically (same horizontal position) but failed when moving horizontally because edge cells didn't have proper padding.

## Possible Future Solutions

### Option 1: Fixed Maximum Padding
- Compute all tiles with maximum expected padding (e.g., padding = 65536 for gen 0-16M)
- **Pros**: Simple, tiles have compatible dimensions
- **Cons**:
  - Massive memory usage (each tile: 256 + 2×65536 = 131,328 cells wide!)
  - First tiles waste huge amounts of memory on unused padding
  - Not practical for exploring deep generations

### Option 2: Padding Re-expansion
- When building Tile N from Tile N-1, first "re-expand" Tile N-1's padding
- Before computing Tile N, take Tile N-1 and compute extra padding cells
- **Pros**: Tiles have correct padding
- **Cons**:
  - Requires computing padding expansion (essentially partial recomputation)
  - Complex buffer management
  - May not save much time vs. recomputing from gen 0

### Option 3: Hierarchical Checkpoints
- Store checkpoint tiles at exponential intervals: gen 0, 1024, 2048, 4096, etc.
- Each checkpoint computes from gen 0 with appropriate padding
- Intermediate tiles compute from nearest checkpoint
- **Pros**:
  - Limits recomputation depth (max 1024 gens from checkpoint)
  - Checkpoints have correct padding for their generation range
- **Cons**:
  - First view still slow (must compute to checkpoint)
  - Checkpoint tiles use more memory (deeper = more padding)
  - Complex cache management

### Option 4: Sparse Tile Storage
- Only cache frequently accessed generation ranges
- Accept that deep generation exploration is slow on first access
- Focus cache on "hot" regions users actually explore
- **Pros**: Simple, leverages cache for typical use cases
- **Cons**: Doesn't solve the deep generation recomputation problem

### Option 5: GPU State Preservation
- Serialize entire GPU buffer state at checkpoints
- Store complete state including padding
- Resume computation from checkpoint state
- **Pros**: Exact state preservation
- **Cons**:
  - Large memory usage for checkpoints
  - Complex serialization/deserialization
  - Unclear if significantly better than recomputing

## Theoretical Considerations

### Is Incremental Computation Even Possible?

The fundamental question: **Can we avoid the padding problem?**

**Key insight**: The padding requirement is not arbitrary—it's mathematically necessary for correct CA computation. A cell's state at generation N depends on cells up to N positions away at generation 0.

**Mathematical constraint**:
```
Cell at position X, generation N
  depends on positions [X-N, X+N] at generation 0
```

Therefore:
- To compute a tile covering positions [A, B] at generation N, we need:
- Initial state covering [A-N, B+N]
- This is why padding = N

**Implication**: Any incremental approach must either:
1. Maintain full padding in all intermediate tiles (Option 1), OR
2. Recompute padding when needed (Option 2), OR
3. Accept limited incremental benefit (Option 3)

There may not be a way to avoid this fundamental trade-off.

## Recommendation for Future Work

Before attempting incremental computation again:

1. **Profile actual use patterns**: How often do users scroll to generation 100,000+?
2. **Measure recomputation cost**: Is computing from gen 0 actually the bottleneck?
3. **Consider GPU speed**: Modern GPUs compute millions of cells/sec. Maybe gen-0 recomputation is "fast enough"?
4. **Evaluate memory vs. speed trade-off**: Would users accept 10x memory usage for 2x speed?

If incremental computation is still desired, **Option 3 (Hierarchical Checkpoints)** seems most promising, but requires careful design of checkpoint intervals and cache eviction strategy.

## Related Ideas

### Alternative: Compressed State Storage
- Store checkpoint states in compressed form
- Decompress and continue computation from checkpoint
- May be faster than recomputation if compression ratio is good

### Alternative: Approximate Caching
- For very deep generations, accept some approximation error
- Use coarser tiles or lower precision computation
- Trade accuracy for speed in less-critical regions

### Alternative: Predictive Pre-computation
- Predict where user will navigate based on movement patterns
- Pre-compute tiles in background thread
- By the time user reaches that region, tile is ready

---

**Last Updated**: 2025-10-24
**Version**: 1.2.0
**Status**: Incremental computation attempted and reverted due to padding incompatibility
