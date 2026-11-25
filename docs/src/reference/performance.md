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
| 81 | 6,561 | 689 | 689 | 0.234 | 6 | 0.2 MiB | 27 B |
| 321 | 103,041 | 10,957 | 10,957 | 1.935 | 6 | 2.7 MiB | 27 B |
| 501 | 251,001 | 14,769 | 14,769 | 7.172 | 6 | 6.3 MiB | 26 B |
| 1001 | 1,002,001 | 75,009 | 75,009 | 43.374 | 6 | 25.8 MiB | 27 B |

### Open-square scaling

These fully open square mazes run from the top-left start to the bottom-right
goal. Values are medians of five separate release-mode runs of the `bench`
example. Times are in ms to 0.001 ms. Timing covers the allocation-instrumented
`Model::trace` call; input parsing is outside that region. The example uses
the same counting allocator as the server. The library itself has no
dependencies and no allocator.

| Side | Cells | Loops | Path length | Time (ms) | Allocations | Peak heap |
|------|-------|-------|-------------|-----------|-------------|-----------|
| 64 | 4,096 | 127 | 127 | 0.104 | 6 | 0.1 MiB |
| 128 | 16,384 | 255 | 255 | 0.481 | 6 | 0.4 MiB |
| 256 | 65,536 | 511 | 511 | 1.717 | 6 | 1.6 MiB |
| 512 | 262,144 | 1,023 | 1,023 | 7.421 | 6 | 6.5 MiB |
| 1024 | 1,048,576 | 2,047 | 2,047 | 35.156 | 6 | 26.0 MiB |

The dedicated `autoresearch` example measures the same 1024×1024 open square
without the counting allocator. Its nine-sample median speed-up is 20,990×;
it reports `solve_ms=0.001` (rounded to 0.001 ms) and `queue_bfs_ms=24.517`.
It validates each route outside the timed batch.

### Comparing solve algorithms

`cargo run --release --example bfs_compare` compares `Model::solve` with
queue-based BFS, bidirectional BFS, and A* on both open-square and deterministic
square backtracker mazes at every open-square sweep size: 64, 128, 256, 512,
and 1024. Backtracker rows are the median over seeds 1–5 of each seed's
median. Maze generation and construction are outside the timed region. A*
uses Manhattan distance for the square-grid heuristic.

| Workload | Cells | `Model::solve` (µs) | Queue BFS (µs) | Bidirectional BFS (µs) | A* (µs) |
|----------|------:|-------------------:|---------------:|-----------------------:|--------:|
| Open square 64×64 | 4,096 | 0.1 | 24.0 | 28.8 | 100.9 |
| Backtracker 64×64 | 4,096 | 9.9 | 6.7 | 8.4 | 13.0 |
| Open square 128×128 | 16,384 | 0.2 | 82.7 | 100.5 | 432.5 |
| Backtracker 128×128 | 16,384 | 38.8 | 28.1 | 39.7 | 55.2 |
| Open square 256×256 | 65,536 | 0.3 | 407.9 | 415.7 | 1,988.6 |
| Backtracker 256×256 | 65,536 | 301.2 | 293.2 | 323.0 | 335.9 |
| Open square 512×512 | 262,144 | 0.6 | 2,953.0 | 4,060.4 | 9,068.8 |
| Backtracker 512×512 | 262,144 | 1,014.1 | 1,095.5 | 1,327.8 | 1,502.7 |
| Open square 1024×1024 | 1,048,576 | 1.3 | 23,641.7 | 20,457.2 | 40,678.0 |
| Backtracker 1024×1024 | 1,048,576 | 10,776.7 | 10,226.9 | 13,612.0 | 14,140.9 |

The comparison sweep was rerun on 2026-11-25. Each sample uses one calibration
solve to choose a repeated batch targeting 10 ms (capped at 1,000,000 solves);
reported timings are per-call batch averages. Per maze, values are medians of
nine samples with solver order rotated, printed to 0.1 µs. `0.0` means below
0.05 µs. Every route's endpoints, traversable cells, edges, and shortest
length are checked outside the timed region. Sub-microsecond timings remain
sensitive to system noise; interpret them as approximate.

With a single goal, `Model::solve` grows waves from both the goal and the
start, widening whichever frontier is smaller, and then rebuilds the route
`Model::trace` would choose. In an earlier run of the same five-seed sweep the
goal-only flood took 2,865.8 µs at 512×512 and 23,859.5 µs at 1024×1024. On
backtracker mazes `solve` was trailing queue BFS. Across seven sweeps the
`solve`/queue BFS ratio stayed near 1.6× at 64×64, 1.5× at 128×128 and 1.3×
at 256×256, moved between 0.8× and 1.1× at 512×512, and ranged 1.18–1.36×
(median 1.30×) at 1024×1024. Marking the route by walking back from the
meeting cells, instead of rescanning every start-side cell, then moved the
ratios over three interleaved runs to 1.71–1.73× at 64×64, 1.53–1.57× at
128×128, 1.19–1.20× at 256×256, 1.01–1.04× at 512×512 and 1.16–1.26× at
1024×1024. Reading the route straight off that backward sweep whenever each
level holds a single route cell, so only a tie needs the forward walk, then
gave 1.41–1.47× at 64×64, 1.32–1.36× at 128×128, 1.03–1.04× at 256×256,
0.90–0.91× at 512×512 and 0.91–1.07× at 1024×1024 over three interleaved
runs. The table above includes both changes; its ratios (1.48×, 1.38×, 1.03×,
0.93× and 1.05×) fall in or next to those ranges. At 1024×1024 run-to-run
noise is about ±10%, so `solve` and queue BFS are effectively tied there.
Storing each start-side cell's parent slot in its spare state bits, so a
tie-free route is read by following one pointer per step instead of scanning
every neighbour, then gave 1.34–1.35× at 64×64, 1.26–1.28× at 128×128,
0.86–0.87× at 256×256, 0.78–0.80× at 512×512 and 0.93–1.02× at 1024×1024
over three interleaved runs (previous version in the same runs: 1.44–1.46×,
1.26–1.36×, 1.02–1.03×, 0.89–0.91× and 1.01–1.07×). The table predates this
change.

## Server: 321×321 boards

Each board was generated once with the backtracker and seed 1, then solved
five times through the API. Server time is the median `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve); it excludes HTTP
and JSON processing.

| Shape | Cells | Path length | Median server time |
|-------|-------|-------------|--------------------|
| Square | 103,041 | 10,957 | 3.691 ms |
| Hexagon | 103,041 | 5,250 | 3.000 ms |
| Triangle | 103,041 | 10,115 | 3.044 ms |
| Penrose rhombs | 126,940 | 4,373 | 2.006 ms |
| Kites and darts | 78,514 | 6,841 | 3.513 ms |

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
grids have wider frontiers. `Model::trace` always runs the fixed-rule flood
from the goals. `Model::solve` constructs a Manhattan-shortest path directly on
a single-goal, fully open square grid, so the open-square `solve` timings above
measure that shortcut; other single-goal mazes use the two-sided waves
described under [Comparing solve algorithms](#comparing-solve-algorithms).
Multi-goal routes can revisit cells while joining shortest-path legs.
