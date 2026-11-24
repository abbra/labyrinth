# Performance Results

Unless noted, measurements below were taken on 2026-11-25 on an x86-64 Linux
workstation with 24 logical CPUs (AMD Ryzen AI 9 HX 370), using release builds.
Timings vary with system load.

## Library benchmarks

### Square backtracker mazes

These mazes use the deterministic square backtracker in
`server/src/generate.rs`, seed 1. Values are medians of five separate
release-mode `bench` runs; times are in ms to 0.001 ms.

| Side | Cells | Loops | Path length | Time (ms) | Allocations | Peak heap | Peak per token |
|------|-------|-------|-------------|-----------|-------------|-----------|----------------|
| 21 | 441 | 85 | 85 | 0.018 | 6 | 0.0 MiB | 28 B |
| 81 | 6,561 | 689 | 689 | 0.217 | 6 | 0.2 MiB | 27 B |
| 321 | 103,041 | 10,957 | 10,957 | 3.296 | 6 | 2.7 MiB | 27 B |
| 501 | 251,001 | 14,769 | 14,769 | 9.963 | 6 | 6.3 MiB | 26 B |
| 1001 | 1,002,001 | 75,009 | 75,009 | 48.350 | 6 | 25.8 MiB | 27 B |

### Open-square scaling

These fully open square mazes run from the top-left start to the bottom-right
goal. Values are medians of five separate release-mode runs of the `bench`
example. Times are in ms to 0.001 ms. Timing covers the allocation-instrumented
`Model::trace` call; input parsing is outside that region. The example uses
the same counting allocator as the server. The library itself has no
dependencies and no allocator.

| Side | Cells | Loops | Path length | Time (ms) | Allocations | Peak heap |
|------|-------|-------|-------------|-----------|-------------|-----------|
| 64 | 4,096 | 127 | 127 | 0.110 | 6 | 0.1 MiB |
| 128 | 16,384 | 255 | 255 | 0.472 | 6 | 0.4 MiB |
| 256 | 65,536 | 511 | 511 | 2.164 | 6 | 1.6 MiB |
| 512 | 262,144 | 1,023 | 1,023 | 9.428 | 6 | 6.5 MiB |
| 1024 | 1,048,576 | 2,047 | 2,047 | 34.561 | 6 | 26.0 MiB |

The dedicated `autoresearch` example measures the same 1024×1024 open square
without the counting allocator. Its nine-sample median speed-up is 17,129×;
it reports `solve_ms=0.001` (rounded to 0.001 ms) and `queue_bfs_ms=23.433`.
It validates each route outside the timed batch.

### Comparing solve algorithms

`cargo run --release --example bfs_compare` compares `Model::solve` with
queue-based BFS, bidirectional BFS, and A* on both open-square and deterministic
square backtracker mazes at every open-square sweep size: 64, 128, 256, 512,
and 1024. Backtracker mazes use seed 1. Maze generation and construction are
outside the timed region. A* uses Manhattan distance for the square-grid
heuristic.

| Workload | Cells | `Model::solve` (µs) | Queue BFS (µs) | Bidirectional BFS (µs) | A* (µs) |
|----------|------:|-------------------:|---------------:|-----------------------:|--------:|
| Open square 64×64 | 4,096 | 0.1 | 24.1 | 28.8 | 100.6 |
| Backtracker 64×64 | 4,096 | 9.9 | 6.3 | 7.5 | 12.4 |
| Open square 128×128 | 16,384 | 0.2 | 79.1 | 98.3 | 422.3 |
| Backtracker 128×128 | 16,384 | 40.1 | 45.9 | 39.4 | 112.2 |
| Open square 256×256 | 65,536 | 0.3 | 383.2 | 415.0 | 2,000.1 |
| Backtracker 256×256 | 65,536 | 506.6 | 335.7 | 305.7 | 442.2 |
| Open square 512×512 | 262,144 | 0.7 | 2,993.1 | 4,079.3 | 9,142.6 |
| Backtracker 512×512 | 262,144 | 2,902.7 | 645.4 | 988.5 | 833.8 |
| Open square 1024×1024 | 1,048,576 | 1.3 | 24,127.7 | 20,684.4 | 40,840.7 |
| Backtracker 1024×1024 | 1,048,576 | 22,398.0 | 16,967.1 | 16,570.2 | 24,418.6 |

The comparison sweep was rerun on 2026-11-25. Each sample uses one calibration
solve to choose a repeated batch targeting 10 ms (capped at 1,000,000 solves);
reported timings are per-call batch averages. Values are medians of nine
samples with solver order rotated, printed to 0.1 µs. `0.0` means below
0.05 µs. Every route's endpoints, traversable cells, edges, and shortest
length are checked outside the timed region. Sub-microsecond timings remain
sensitive to system noise; interpret them as approximate.

## Server: 321×321 boards

Each board was generated once with the backtracker and seed 1, then solved
five times through the API. Server time is the median `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve); it excludes HTTP
and JSON processing.

| Shape | Cells | Path length | Median server time |
|-------|-------|-------------|--------------------|
| Square | 103,041 | 10,957 | 3.866 ms |
| Hexagon | 103,041 | 5,250 | 3.269 ms |
| Triangle | 103,041 | 10,115 | 2.875 ms |
| Penrose rhombs | 126,940 | 4,373 | 2.512 ms |
| Kites and darts | 78,514 | 6,841 | 3.871 ms |

## Memory use

The values below are medians of five solves of the same generated boards. The
server reports allocation statistics in the `heap` object in
[POST /api/solve](../user-guide/http-api.md#post-apisolve).

| Shape | Tokens | Loops | Allocations | Allocated | Peak | Peak per token |
|-------|--------|-------|-------------|-----------|------|----------------|
| Square | 103,041 | 10,957 | 6 | 2.7 MiB | 2.7 MiB | 27 B |
| Hexagon | 103,041 | 5,250 | 6 | 2.6 MiB | 2.6 MiB | 27 B |
| Triangle | 103,041 | 10,115 | 6 | 2.7 MiB | 2.7 MiB | 27 B |
| Penrose rhombs | 126,940 | 4,373 | 6 | 3.2 MiB | 3.2 MiB | 27 B |
| Kites and darts | 78,514 | 6,841 | 6 | 2.0 MiB | 2.0 MiB | 27 B |

Allocation counts are independent of loop count: the solver allocates its
working buffers once and reuses them. These measurements use sequential
requests; the counters are process-wide, so concurrent solves can affect
reported heap statistics.

## Time for one solve

For the general BFS path, each frontier step advances one graph-distance
layer. Long narrow corridors need many steps with little work per step; open
grids have wider frontiers. `Model::trace` always runs the fixed-rule flood.
`Model::solve` instead constructs a Manhattan-shortest path directly on a
single-goal, fully open square grid, so the open-square `solve` timings above
measure that shortcut. Multi-goal routes can revisit cells while joining
shortest-path legs.
