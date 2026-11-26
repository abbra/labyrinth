# Performance Results

The current release sweep was run on 2026-11-26 on an AMD Ryzen AI 9 HX 370
(x86-64 Linux, rustc 1.100.0). Solver comparisons were pinned to logical CPU 2.
The transformer used runtime-selected AVX2; no target-feature flags were set.

## Library benchmarks

### Square backtracker mazes

These mazes use the deterministic square backtracker in
`server/src/generate.rs`, seed 1. Values are medians of five separate
release-mode `bench` runs on CPU 4 (`taskset -c 4 target/release/examples/bench < maze.txt`).
The timer covers `Model::trace`, not parsing; path length counts cells, including
both endpoints. Heap sizes rounded to 0.0 MiB are not zero allocations.

| Side | Cells | Loops | Path length | Time (ms) | Allocations | Peak heap | Peak per token |
|------|-------|-------|-------------|-----------|-------------|-----------|----------------|
| 21 | 441 | 85 | 85 | 0.018 | 6 | 0.0 MiB | 28 B |
| 81 | 6,561 | 689 | 689 | 0.226 | 6 | 0.2 MiB | 27 B |
| 321 | 103,041 | 10,957 | 10,957 | 5.614 | 6 | 2.7 MiB | 27 B |
| 501 | 251,001 | 14,769 | 14,769 | 14.427 | 6 | 6.3 MiB | 26 B |
| 1001 | 1,002,001 | 75,009 | 75,009 | 65.385 | 6 | 25.8 MiB | 27 B |

### Open-square scaling

These fully open square mazes run from the top-left start to the bottom-right
goal. Values are medians of five separate release-mode runs of the `bench`
example on CPU 4. Times are in ms to 0.001 ms. Timing covers the allocation-instrumented
`Model::trace` call; input parsing is outside that region. The example uses
the same counting allocator as the server. The library itself has no
dependencies and no allocator.

| Side | Cells | Loops | Path length | Time (ms) | Allocations | Peak heap |
|------|-------|-------|-------------|-----------|-------------|-----------|
| 64 | 4,096 | 127 | 127 | 0.112 | 6 | 0.1 MiB |
| 128 | 16,384 | 255 | 255 | 0.482 | 6 | 0.4 MiB |
| 256 | 65,536 | 511 | 511 | 2.352 | 6 | 1.6 MiB |
| 512 | 262,144 | 1,023 | 1,023 | 13.114 | 6 | 6.5 MiB |
| 1024 | 1,048,576 | 2,047 | 2,047 | 45.968 | 6 | 26.0 MiB |

The dedicated `autoresearch` example measures the same 1024×1024 open square
without the counting allocator. Its nine-sample median speed-up is 18,066×;
it reports `solve_ms=0.001` (rounded to 0.001 ms) and `queue_bfs_ms=22.961`.
It validates each route outside the timed batch.

### Running the shared algorithm sweep

`cargo run --release --example bfs_compare` runs seven methods on the same
16×16 and 32×32 boards: `Model`, queue BFS, bidirectional BFS, Manhattan A*,
both transformers and the Gholami–Hajiaghayi scan base case. The default is
nine rotated samples, an open board and seed 1 for both backtracker workloads.
Results are per-board medians in microseconds; seeds are reported separately.

```sh
# All powers of two from 16 through 1024, all three workloads, one sample per method
taskset -c 2 cargo run --release --example bfs_compare -- --sizes all --runs 1
# Select just the endpoint or a subset without running the intermediate sizes
cargo run --release --example bfs_compare -- --sizes 1024 --workload open --runs 1
cargo run --release --example bfs_compare -- --sizes 32,64 --workload multi --seeds 5
```

Large transformer solves can be expensive: each layer scans the board, and
a long four-goal route needs many layers. Nothing is skipped at large sizes.
One board is benchmarked at a time. The first timed solve sets the batch
length and counts as a sample by itself if no repetitions are needed.
On four-goal backtrackers, the point-to-point algorithms search each remaining
target and take the shortest leg, breaking ties by neighbour-slot order.
This extra target-selection work is timed, not supplied by the BFS oracle.

### Weighted single-source search on unit-edge mazes

[`gh_scan_base` implements Algorithm 8, lines 3–6](https://arxiv.org/html/2611.34911v1#algorithm8).
The problem in the paper is distances from one source to **all** reachable vertices.
Its randomized hierarchy is never selected here: the base case applies when
*N* ≤ ⌈log₂(*N* + 2)⌉⁶⁴, including every supported maze. Our edges are all
positive and unit length, so there are no zero-edge components to contract.
Each leg scans finite unsettled labels for Dijkstra's minimum until none remain,
then builds the entire predecessor tree by traversing tight edges in queue and
neighbour-slot order. The first remaining goal in this traversal supplies the leg;
neither pass stops there. Unreachable cells retain infinite distance.

Median µs per solve, five rotated calibrated batches on CPU 2, seed 1 for backtrackers
(`--sizes 32,64 --runs 5`). All six boards matched the queue BFS reference route.

| Side | Board | GH scan base | Queue BFS | Model |
|---:|---|---:|---:|---:|
| 32 | Open | 26.955 | 4.740 | 0.045 |
| 32 | Single-goal backtracker | 6.179 | 1.634 | 1.634 |
| 32 | Four-goal backtracker | 25.043 | 3.468 | 2.972 |
| 64 | Open | 119.494 | 17.663 | 0.105 |
| 64 | Single-goal backtracker | 28.949 | 6.462 | 6.982 |
| 64 | Four-goal backtracker | 156.231 | 25.144 | 22.698 |

The useful idea is to separate distance computation from deterministic readout:
a tight edge advances exactly one BFS level here. `Model` already uses that
condition to resolve tied meeting paths without building a full source tree.
The transformer records the first eligible downhill slot at wave arrival; a
separate tree pass would duplicate that decision and would not reduce layer
count. These results compare the base case, **not** the weighted hierarchy.

### Shared sweep results

This earlier sweep predates the seventh method.

Times are microseconds per complete solve, including packing but excluding maze and
solver construction. Each entry is one calibrated batch, not a multi-run median.
Both backtracker workloads use seed 1. Routes were checked against queue BFS
outside the timer; single-target A* and bidirectional BFS may return a different
valid shortest path. Steps count edges, including revisits between goals.

#### Open square

| Side | Steps | Model | Queue BFS | Bidirectional BFS | A* | Dense transformer | Wall-skipping |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 16 | 30 | 0.039 | 1.632 | 1.898 | 4.481 | 28.763 | 28.567 |
| 32 | 62 | 0.073 | 6.368 | 7.322 | 23.533 | 227.660 | 224.287 |
| 64 | 126 | 0.097 | 17.208 | 23.148 | 77.669 | 1,307.512 | 1,325.268 |
| 128 | 254 | 0.166 | 87.903 | 100.850 | 435.314 | 11,382.220 | 11,173.969 |
| 256 | 510 | 0.282 | 401.974 | 561.051 | 2,026.624 | 94,167.213 | 106,153.158 |
| 512 | 1,022 | 0.549 | 3,280.320 | 5,590.026 | 9,215.665 | 786,307.056 | 762,554.075 |
| 1024 | 2,046 | 1.157 | 24,449.130 | 25,450.173 | 43,252.260 | 7,941,636.250 | 7,675,718.618 |

#### Single-goal backtracker

| Side | Steps | Model | Queue BFS | Bidirectional BFS | A* | Dense transformer | Wall-skipping |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 16 | 48 | 0.691 | 0.715 | 0.678 | 1.092 | 44.666 | 18.962 |
| 32 | 148 | 2.095 | 1.998 | 2.222 | 3.340 | 435.334 | 175.789 |
| 64 | 532 | 6.731 | 6.213 | 7.512 | 13.028 | 5,426.768 | 2,482.206 |
| 128 | 2,128 | 30.221 | 48.705 | 40.739 | 118.905 | 93,024.423 | 41,215.949 |
| 256 | 8,192 | 264.464 | 345.443 | 344.078 | 464.817 | 1,462,386.599 | 665,285.128 |
| 512 | 16,168 | 604.002 | 666.642 | 955.421 | 859.096 | 12,876,648.949 | 5,723,830.578 |
| 1024 | 68,176 | 13,149.515 | 16,511.088 | 18,496.633 | 23,624.208 | 266,718,459.321 | 122,528,564.667 |

#### Four-goal backtracker

| Side | Steps | Model | Queue BFS | Bidirectional BFS | A* | Dense transformer | Wall-skipping |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 16 | 68 | 1.214 | 1.382 | 4.873 | 7.444 | 63.590 | 28.154 |
| 32 | 252 | 2.924 | 3.345 | 13.953 | 23.107 | 661.798 | 281.913 |
| 64 | 1,584 | 21.743 | 23.895 | 88.918 | 245.904 | 16,758.383 | 7,521.498 |
| 128 | 2,900 | 68.672 | 82.145 | 420.596 | 1,099.361 | 128,927.558 | 56,802.759 |
| 256 | 14,164 | 999.948 | 1,071.563 | 3,726.544 | 6,431.468 | 2,571,848.050 | 1,189,108.491 |
| 512 | 55,516 | 8,807.186 | 10,161.314 | 35,800.602 | 54,139.419 | 46,273,408.889 | 19,363,435.728 |
| 1024 | 153,472 | 54,000.274 | 60,504.720 | 152,988.793 | 281,185.963 | 582,752,915.445 | 260,246,040.361 |

### Why weight layout matters

The comparisons below are earlier controlled kernel experiments, not timings
from the current release sweep.

The default x86-64 binary checks for AVX2 once per `Transformer::solve` call,
outside the cell loop. On AVX2, each lane represents one cell. The solver
packs neighbour indices and softmax weights once per solve, then gathers from
the previous layer's reached channels. Pointer updates stay local to each
cell. Other CPUs and mazes beyond the signed gather-index range use the
portable kernel. No `RUSTFLAGS` are needed.

On the same 32×32 open, backtracker and four-goal workloads, cell SIMD took
213 / 456 / 796 µs versus 847 / 2,105 / 3,293 µs for head SIMD, including
packing time. That version duplicated 256 bytes of weights alongside 32 bytes
of indices per occupied slot in each eight-cell batch. The current kernel keeps
the indices per batch but shares weights across batches with the same ordered
neighbour counts. Only setup uses a hash table; each layer reads the shared
weights by offset.

On a 512×512 open square, six degree patterns need only 5.25 KiB of packed
weight data instead of nearly 32 MiB. In a pinned before/after comparison,
median solve time fell from 1.650 to 0.818 seconds (three samples, packing
included). The fraction of L2 fills served by local memory fell from 79.9%
to 23.6% (`l2_fill_rsp_src.dram_io_near / l2_fill_rsp_src.all`).
The seed-1 backtracker fell from 25.98 to 13.65 seconds and its four-goal
variant from 98.43 to 48.01 seconds (one sample each). Irregular graphs with
few repeated degree patterns need not see the same savings.

On irregular graphs the order of those patterns matters. The packer checks reuse
after the first 8 MiB of weights and after each additional 64 KiB. If hits within
64 KiB of the write position saved at least half a sample's worth of weight rows,
it keeps the lookup table; otherwise it clears the table so distant repeats get
new copies in traversal order. Existing offsets remain valid. These are sampling
intervals, not bounds on total allocation.

On a 262,144-cell graph with degrees 1–8 in every batch, scattering 16,384
patterns (two uses each) took 179 ms with per-batch weights, 271 ms with global
sharing, and 172 ms with fixed 64 KiB windows. Placing identical patterns together
instead took 182 / 109 / 119 ms respectively. On a 512×512 open square the
same kernels took 1,626 / 814 / 818 ms. Times are five-solve medians on one
pinned Ryzen AI 9 HX 370 core, including packing; all routes matched queue BFS.

In a nine-pass comparison, adapting those windows reduced the median for
16,384 clustered patterns from 119.8 to 110.1 ms (global sharing: 111.4 ms).
Scattered pairs took 175.5 versus 175.4 ms with fixed windows; on the open
512×512 square, adaptive took 821 versus 812 ms. All timed routes matched BFS.

Shuffling already packed patterns can stop table growth entirely. Once packed
weights exceed 8 MiB, the packer also counts hits that jump more than 64 KiB
from the preceding batch's offset. At each 64-batch boundary it clears the
lookup if more than 32 batches made such jumps since the last boundary or clear.
On adjacent pairs followed by shuffled reads of the same 8,192 patterns, this
trigger fired 64 batches after the switch. Median solve time over nine passes
fell from 215.3 to 147.1 ms (fixed windows: 142.8 ms). On 16,384 clustered
pairs the trigger never fired: 108.5 ms versus 110.5 ms without it and 119.2 ms
with fixed windows. These thresholds describe one CPU and one graph topology.

On the same graph, alternating clustered and scattered reads every 32 batches
exposed the cost of bypassing the lookup but retaining old offsets. When
clustered lookup resumed, consecutive batches jumped more than 64 KiB about
8,200 times per traversal, versus about 1,650 in production. In phase-bracketed
counter runs, bypass saved roughly 5% of packing cycles but incurred 4.2–4.3×
as many layer demand L2 misses. Repacking distant hits brought misses back near
production, but increased weight storage from 40.3 to 49.3 MiB (production:
46.4 MiB). None of twelve repack thresholds from 0 to 16 MiB beat production
in either starting order. In twenty paired validation runs the best compromise,
64 KiB, was still 3.2% slower clustered-first and 2.6% slower scattered-first.
Production keeps its adaptive lookup windows; it neither bypasses the table
during scattered phases nor relocates hits when clustered lookup resumes.

The transformer evaluates attention and the MLP on every cell for each distance
layer of each leg, including walls. Queue BFS processes only its frontier.
`Model::solve` bypasses search entirely on these open squares; its sub-microsecond
timings should be treated as approximate.

### Comparing solve algorithms

The historical classical sweep below compared `Model::solve` with queue BFS,
bidirectional BFS, and Manhattan A* on open squares and deterministic
backtrackers. Backtracker rows are the median over seeds 1–5 of each seed's
median. Maze generation and construction were outside the timed region.

| Workload | Cells | `Model::solve` (µs) | Queue BFS (µs) | Bidirectional BFS (µs) | A* (µs) |
|----------|------:|-------------------:|---------------:|-----------------------:|--------:|
| Open square 64×64 | 4,096 | 0.2 | 23.7 | 28.6 | 101.3 |
| Backtracker 64×64 | 4,096 | 7.0 | 6.6 | 8.1 | 12.8 |
| Open square 128×128 | 16,384 | 0.2 | 80.1 | 97.7 | 418.8 |
| Backtracker 128×128 | 16,384 | 28.4 | 28.3 | 38.5 | 54.8 |
| Open square 256×256 | 65,536 | 0.4 | 414.5 | 411.6 | 1,955.6 |
| Backtracker 256×256 | 65,536 | 238.0 | 293.5 | 313.9 | 346.4 |
| Open square 512×512 | 262,144 | 0.7 | 2,579.7 | 3,800.2 | 8,792.0 |
| Backtracker 512×512 | 262,144 | 794.8 | 1,089.1 | 1,471.7 | 1,507.1 |
| Open square 1024×1024 | 1,048,576 | 1.3 | 23,855.6 | 21,210.2 | 41,101.2 |
| Backtracker 1024×1024 | 1,048,576 | 8,809.5 | 10,193.7 | 13,542.3 | 13,843.3 |

Each sample uses one calibration
solve to choose a repeated batch targeting 10 ms (capped at 1,000,000 solves);
reported timings are per-call batch averages. Per maze, values are medians of
nine samples with solver order rotated, printed to 0.1 µs. `0.0` means below
0.05 µs. Every route's endpoints, traversable cells, edges, and shortest
length are checked outside the timed region. Sub-microsecond timings remain
sensitive to system noise; interpret them as approximate.

On open squares the comparison measures direct route construction, not a faster
graph search. On single-goal backtrackers, `Model::solve` and queue BFS were
within 7% at sides 64 and 128. At sides 256, 512 and 1024 the model took
81%, 73% and 86% of queue BFS time, respectively. See
[How the Model Works](model.md) for the two-sided search and route readout.

### Several goals

`bfs_compare` also times each backtracker maze with three extra goals, in the
rooms next to the other two corners and in the centre room. The baseline,
nearest-goal BFS, runs one queue BFS leg from the current cell to the nearest
unvisited goal until every goal is visited, which is the route `Model::solve`
returns. Both routes are checked for endpoints, edges, length and goal
coverage outside the timed region. This historical comparison did not run
bidirectional BFS or A* on multiple goals. Settings and seeds match the table above.
The ratio is model time divided by nearest-goal BFS time (lower is better).

| Workload | Cells | `Model::solve` (µs) | Nearest-goal BFS (µs) | Ratio |
|----------|------:|-------------------:|----------------------:|------:|
| Backtracker 64×64, 4 goals | 4,096 | 16.8 | 19.1 | 0.88 |
| Backtracker 128×128, 4 goals | 16,384 | 73.2 | 86.2 | 0.85 |
| Backtracker 256×256, 4 goals | 65,536 | 845.6 | 892.6 | 0.95 |
| Backtracker 512×512, 4 goals | 262,144 | 5,731.3 | 6,278.2 | 0.91 |
| Backtracker 1024×1024, 4 goals | 1,048,576 | 37,660.6 | 40,691.6 | 0.93 |

With several goals `solve` grows waves from the start and from all goals at
once, as in the single-goal search, and stops at the first contact. It reads
the route to the goal that was reached and then runs nearest-goal BFS legs for
the remaining goals, like the baseline.

## Server: 321×321 boards

Each board was generated once with the backtracker and seed 1, then solved
five times through the API, with the release server pinned to one core
(`taskset -c 4`). Server time is the median `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve); it excludes HTTP
and JSON processing. The endpoint runs `Model::trace`, so these times do not
include the `Model::solve` fast paths.

| Shape | Cells | Path length | Median server time |
|-------|-------|-------------|--------------------|
| Square | 103,041 | 10,957 | 3.590 ms |
| Hexagon | 103,041 | 5,250 | 2.814 ms |
| Triangle | 103,041 | 10,115 | 2.940 ms |
| Penrose rhombs | 126,940 | 4,373 | 2.052 ms |
| Kites and darts | 78,514 | 6,841 | 3.332 ms |

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
