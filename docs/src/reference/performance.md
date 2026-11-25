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

### Full transformer versus optimized solve and queue BFS

Reproduce with `taskset -c 2 cargo run --release --example bfs_compare -- --transformer`.
Times below are median microseconds per complete route over nine calibrated
batches, rotating solver order. All three solvers used the same maze; construction
of the maze and weights was excluded. Each sample's returned path was checked
for exact equality with queue BFS outside the timer. Backtracker rows use seed 1;
the four-goal baseline repeats BFS to the nearest unvisited goal.

| Maze | Goals | Steps | Previous transformer (µs) | Fused transformer (µs) | `Model::solve` (µs) | Queue BFS (µs) |
|------|------:|------:|--------------------------:|-----------------------:|--------------------:|---------------:|
| Open 16×16 | 1 | 30 | 637.599 | 143.898 | 0.028 | 1.209 |
| Backtracker 16×16 | 1 | 48 | 1,024.111 | 232.396 | 0.457 | 0.464 |
| Backtracker 16×16 | 4 | 68 | 1,506.273 | 341.428 | 0.835 | 1.015 |
| Open 32×32 | 1 | 62 | 5,132.810 | 1,169.279 | 0.048 | 4.428 |
| Backtracker 32×32 | 1 | 148 | 12,324.233 | 2,788.006 | 1.578 | 1.592 |
| Backtracker 32×32 | 4 | 252 | 20,985.182 | 4,643.779 | 2.893 | 3.313 |

Separating the ordered prefix sum from the eight pointer updates lets the
compiler emit packed `f32` ReLUs (`maxps` in the x86-64 release build) without
changing summation order or requiring target-specific intrinsics. In a pinned
before/after run, the 32×32 open, backtracker, and four-goal backtracker cases
fell from 1,186 / 2,788 / 4,678 µs to 903 / 2,186 / 3,571 µs respectively.

The default x86-64 binary checks for AVX2 once per `Transformer::solve` call,
outside the cell loop. When available, it accumulates all eight attention heads
in one 256-bit register. Other CPUs use the portable kernel. No `RUSTFLAGS` are
needed; globally enabling AVX2 would make the binary unsuitable for older CPUs.

With runtime dispatch on the AVX2 host, the three 32×32 workloads above took
846 / 2,096 / 3,294 µs using the same pinned benchmark command.

The transformer evaluates attention and the MLP on every cell for each distance
layer of each leg, including walls. Queue BFS processes only its frontier.
`Model::solve` bypasses search entirely on these open squares; its sub-microsecond
timings should be treated as approximate.

### Comparing solve algorithms

`cargo run --release --example bfs_compare` compares `Model::solve` with
queue-based BFS, bidirectional BFS, and A* on both open-square and deterministic
square backtracker mazes at every open-square sweep size: 64, 128, 256, 512,
and 1024. Backtracker rows are the median over seeds 1–5 of each seed's
median. Maze generation and construction are outside the timed region. A*
uses Manhattan distance for the square-grid heuristic.

| Workload | Cells | `Model::solve` (µs) | Queue BFS (µs) | Bidirectional BFS (µs) | A* (µs) |
|----------|------:|-------------------:|---------------:|-----------------------:|--------:|
| Open square 64×64 | 4,096 | 0.1 | 17.5 | 21.7 | 73.6 |
| Backtracker 64×64 | 4,096 | 7.1 | 6.8 | 8.3 | 13.0 |
| Open square 128×128 | 16,384 | 0.2 | 82.9 | 99.6 | 425.5 |
| Backtracker 128×128 | 16,384 | 28.8 | 28.4 | 38.7 | 55.6 |
| Open square 256×256 | 65,536 | 0.4 | 396.2 | 416.1 | 1,958.3 |
| Backtracker 256×256 | 65,536 | 240.9 | 291.4 | 317.9 | 335.2 |
| Open square 512×512 | 262,144 | 0.7 | 2,504.8 | 3,903.3 | 8,900.3 |
| Backtracker 512×512 | 262,144 | 802.6 | 1,114.3 | 1,364.3 | 1,486.6 |
| Open square 1024×1024 | 1,048,576 | 1.3 | 24,268.8 | 21,155.4 | 40,476.4 |
| Backtracker 1024×1024 | 1,048,576 | 8,270.1 | 9,635.1 | 13,846.0 | 13,222.6 |

The comparison sweep was rerun on 2026-11-25, pinned to one CPU core with
`taskset -c 2`. Each sample uses one calibration
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
runs. At 1024×1024 run-to-run noise is about ±10%, so `solve` and queue BFS
are effectively tied there. Storing each start-side cell's parent slot in its
spare state bits, so a tie-free route is read by following one pointer per
step instead of scanning every neighbour, then gave 1.34–1.35× at 64×64,
1.26–1.28× at 128×128, 0.86–0.87× at 256×256, 0.78–0.80× at 512×512 and
0.93–1.02× at 1024×1024 over three interleaved runs (previous version in the
same runs: 1.44–1.46×, 1.26–1.36×, 1.02–1.03×, 0.89–0.91× and 1.01–1.07×).
An earlier table, which included those three changes, showed ratios of 1.34×,
1.28×, 0.87×, 0.82× and 0.97×, in or next to those ranges. Recording each
start-side cell's parent as an index into the wave-order list, instead of a
neighbour slot, drops the slot lookup during the search and makes each route
step one dependent load. Over three interleaved runs that gave 1.16–1.21× at
64×64, 1.07–1.12× at 128×128, 0.85–0.87× at 256×256, 0.76–0.78× at 512×512
and 0.82–0.91× at 1024×1024 (previous version in the same runs: 1.34–1.35×,
1.25–1.31×, 0.87–0.89×, 0.79–0.82× and 0.94–0.98×). An intermediate table
with all four changes showed 1.16×, 1.12×, 0.86×, 0.77× and 0.81×. Reserving
the wave-order and parent lists for every cell up front, instead of growing
them, and inlining the goal-side wave step, which corridor mazes run about once
per cell, then gave 1.06× at 64×64, 0.99–1.01× at 128×128, 0.83–0.84× at
256×256, 0.74× at 512×512 and 0.80–0.92× at 1024×1024 over three interleaved
runs (previous version in the same runs: 1.16–1.17×, 1.09–1.10×, 0.86–0.87×,
0.76–0.77× and 0.89–0.91×). Most of the small-maze gain came from the
reservation; inlining alone gave about 2–3%. The reservation costs 12 bytes per
cell. The table above includes all five changes; its ratios are 1.04×, 1.01×,
0.83×, 0.72× and 0.86×.

### Several goals

`bfs_compare` also times each backtracker maze with three extra goals, in the
rooms next to the other two corners and in the centre room. The baseline,
nearest-goal BFS, runs one queue BFS leg from the current cell to the nearest
unvisited goal until every goal is visited, which is the route `Model::solve`
returns. Both routes are checked for endpoints, edges, length and goal
coverage outside the timed region. Bidirectional BFS and A* need a single
target, so they are not run. Same settings and seeds as the table above; each
time is the median of three interleaved runs pinned to one core.

| Workload | Cells | `Model::solve` (µs) | Nearest-goal BFS (µs) | Ratio |
|----------|------:|-------------------:|----------------------:|------:|
| Backtracker 64×64, 4 goals | 4,096 | 19.1 | 23.1 | 0.83 |
| Backtracker 128×128, 4 goals | 16,384 | 81.8 | 103.8 | 0.79 |
| Backtracker 256×256, 4 goals | 65,536 | 908.9 | 1,027.7 | 0.88 |
| Backtracker 512×512, 4 goals | 262,144 | 5,696.3 | 6,231.7 | 0.91 |
| Backtracker 1024×1024, 4 goals | 1,048,576 | 36,244.6 | 42,360.3 | 0.86 |

With several goals `solve` grows waves from the start and from all goals at
once, as in the single-goal search, and stops at the first contact. It reads
the route to the goal that was reached and then runs nearest-goal BFS legs for
the remaining goals, like the baseline. Before this change `solve` flooded
from the goals until the flood reached the start. That flood covered every
open cell in 9 of the 10 mazes, and `solve` was 1.2–2.4× slower than the
baseline.

## Server: 321×321 boards

Each board was generated once with the backtracker and seed 1, then solved
five times through the API, with the release server pinned to one core
(`taskset -c 2`). Server time is the median `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve); it excludes HTTP
and JSON processing. The endpoint runs `Model::trace`, so these times do not
include the `Model::solve` fast paths.

| Shape | Cells | Path length | Median server time |
|-------|-------|-------------|--------------------|
| Square | 103,041 | 10,957 | 2.386 ms |
| Hexagon | 103,041 | 5,250 | 1.526 ms |
| Triangle | 103,041 | 10,115 | 1.535 ms |
| Penrose rhombs | 126,940 | 4,373 | 1.299 ms |
| Kites and darts | 78,514 | 6,841 | 2.262 ms |

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
