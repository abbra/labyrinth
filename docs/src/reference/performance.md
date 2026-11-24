# Performance Results

All values come from one machine with 24 cores and release builds.

## Library: square mazes

These mazes come from a square backtracker generator.

| Side | Loops | Time |
|------|-------|------|
| 321 | 4,893 | 0.280 s |
| 1001 | 67,073 | 2.518 s |

## Server: 321×321 boards

The server time is the `micros` value from
[POST /api/solve](../user-guide/http-api.md#post-apisolve).

| Shape | Cells | Path length | Server time |
|-------|-------|-------------|-------------|
| Square | 103,041 | 10,957 | 291 ms |
| Hexagon | 103,041 | 5,250 | 214 ms |
| Triangle | 103,041 | 10,115 | 166 ms |
| Penrose rhombs | 126,940 | 4,373 | 104 ms |
| Kites and darts | 78,514 | — | 299 ms (6,841 loops) |

## Memory use

The server counts heap allocations during each solve. The values come from
the `heap` object in [POST /api/solve](../user-guide/http-api.md#post-apisolve).
The boards are 321×321 backtracker mazes, seed 1.

| Shape | Tokens | Loops | Allocations | Allocated | Peak | Peak per token |
|-------|--------|-------|-------------|-----------|------|----------------|
| Square | 103,041 | 10,957 | 108 | 22.9 MiB | 22.8 MiB | 232 B |
| Hexagon | 103,041 | 5,250 | 109 | 23.4 MiB | 23.3 MiB | 237 B |
| Triangle | 103,041 | 10,115 | 109 | 22.8 MiB | 22.7 MiB | 231 B |
| Penrose rhombs | 126,940 | 4,373 | 108 | 29.1 MiB | 29.0 MiB | 239 B |
| Kites and darts | 78,514 | 6,841 | 108 | 18.3 MiB | 18.2 MiB | 243 B |

The number of allocations does not change with the number of loops. The
solver allocates its buffers one time and then uses them again in each
loop. The peak is approximately 235 bytes for each token. Thus the memory
use is linear in the number of cells.

The counters are process-wide. Allocations by other requests that run at
the same time are also counted.

## Speed-ups over the first server version

The first server version evaluated all tokens in each loop. The current
version evaluates only the tokens near a change, stops early and uses a
thread pool.

| Side | Old | New | Speed-up |
|------|-----|-----|----------|
| 81 | 5.88 s | 0.179 s | 33× |
| 121 | 53.7 s | 0.362 s | 148× |
| 161 | 141 s | 0.652 s | 217× |
| 241 | 268 s | 1.40 s | 191× |

## Time for one solve

The number of loops is equal to the path length. The work in one loop is
proportional to the size of the search front. Thus long, narrow corridors
need many loops with little work in each loop. Open caves need few loops
with more work in each loop.
