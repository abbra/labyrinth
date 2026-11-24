# Performance Results

Measurements below were taken on 2026-11-24 on an x86-64 Linux workstation
with 24 logical CPUs (AMD Ryzen AI 9 HX 370), using release builds. Timings
vary with system load.

## Library benchmarks

### Square backtracker mazes

These mazes use the deterministic square backtracker in
`server/src/generate.rs`, seed 1. Values are medians of five separate
release-mode `bench` runs; times are printed to 1 ms precision.

| Side | Cells | Loops | Path length | Time | Allocations | Peak heap | Peak per token |
|------|-------|-------|-------------|------|-------------|-----------|----------------|
| 21 | 441 | 85 | 85 | 0.000 s | 6 | 0.0 MiB | 28 B |
| 81 | 6,561 | 689 | 689 | 0.000 s | 6 | 0.2 MiB | 27 B |
| 321 | 103,041 | 10,957 | 10,957 | 0.003 s | 6 | 2.7 MiB | 27 B |
| 501 | 251,001 | 14,769 | 14,769 | 0.012 s | 6 | 6.3 MiB | 26 B |
| 1001 | 1,002,001 | 75,009 | 75,009 | 0.049 s | 6 | 25.8 MiB | 27 B |

### Open-square scaling

These fully open square mazes run from the top-left start to the bottom-right
goal. The table reports medians of five separate release-mode runs of the
`bench` example. Its timer covers `Model::trace`; input parsing is outside the
timed region. The example uses the same counting allocator as the server.
The library itself has no dependencies and no allocator.

| Side | Cells | Loops | Path length | Time | Allocations | Peak heap |
|------|-------|-------|-------------|------|-------------|-----------|
| 64 | 4,096 | 127 | 127 | 0.000 s | 6 | 0.1 MiB |
| 128 | 16,384 | 255 | 255 | 0.001 s | 6 | 0.4 MiB |
| 256 | 65,536 | 511 | 511 | 0.002 s | 6 | 1.6 MiB |
| 512 | 262,144 | 1,023 | 1,023 | 0.008 s | 6 | 6.5 MiB |
| 1024 | 1,048,576 | 2,047 | 2,047 | 0.034 s | 6 | 26.0 MiB |

The dedicated `autoresearch` example measures the same 512×512 maze without
the counting allocator. The initial baseline was 158.17 ms; the current
three-run median is 6.000 ms, a 26.4× speed-up. Both report 1,023 loops and
1,048,571 token passes.

## Server: 321×321 boards

Each board was generated once with the backtracker and seed 1, then solved
five times through the API. Server time is the median `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve); it excludes HTTP
and JSON processing.

| Shape | Cells | Path length | Median server time |
|-------|-------|-------------|--------------------|
| Square | 103,041 | 10,957 | 3.817 ms |
| Hexagon | 103,041 | 5,250 | 1.917 ms |
| Triangle | 103,041 | 10,115 | 1.461 ms |
| Penrose rhombs | 126,940 | 4,373 | 2.101 ms |
| Kites and darts | 78,514 | 6,841 | 1.973 ms |

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

For a single goal, each loop advances the search frontier by one layer, so
long narrow corridors need many loops with little work per loop. Open grids
need fewer loops but process a wider frontier. Multi-goal routes can revisit
cells while joining shortest-path legs.
