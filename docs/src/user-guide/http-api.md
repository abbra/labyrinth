# HTTP API

Handler validation errors return status `400` with a plain-text body. Axum
rejects malformed JSON and invalid enum values, such as an unknown shape or
algorithm, with status `422`.

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/` | The browser UI |
| `GET` | `/api/model` | Model data |
| `POST` | `/api/generate` | Make a maze |
| `POST` | `/api/solve` | Solve a maze |

## GET /api/model

Response:

```json
{ "parameters": 1472, "max_side": 321 }
```

## POST /api/generate

Request:

```json
{ "shape": "hex", "algorithm": "backtracker", "width": 15, "height": 15, "seed": 42 }
```

| Field | Values |
|-------|--------|
| `shape` | `square`, `hex`, `triangle`, `penrose`, `kite-dart` |
| `algorithm` | `backtracker`, `prim`, `braid`, `caves` |
| `width`, `height` | 5 to 321 |
| `seed` | Any unsigned 64-bit integer. The same seed gives the same maze |

Response:

```json
{ "cells": "##S..#...", "polygons": [[0.866, 0.5, 0.0, 1.0, ...], ...] }
```

| Field | Meaning |
|-------|---------|
| `cells` | One character for each cell: `#`, `.`, `S` or `G`. The index of a character is the cell id |
| `polygons` | The outline of each cell, as `[x0, y0, x1, y1, ...]`. The value is `null` for square boards |

For a square board, the cell id of row `r`, column `c` is
`r * width + c`. The client calculates the squares itself.

The tilings and the generators are described in
[Tilings and Generators](tilings.md).

## POST /api/solve

Request:

```json
{ "shape": "hex", "width": 15, "height": 15, "cells": "##S..#..." }
```

Send the same `shape`, `width` and `height` that you used to generate the
board. The server makes the board again and uses it to find the
neighbours. You can change `cells` before you send it. For example, you
can add walls or move the start.

The endpoint calls `Model::trace` to return `reached_at` and flood statistics.
It therefore runs the fixed-rule flood even for a single-goal open square;
the direct-path shortcut is specific to `Model::solve`.

Response:

| Field | Meaning |
|-------|---------|
| `path` | The cell ids from start through every goal, or `null` if any goal is unreachable |
| `reached_at` | For each cell: the loop in which the search reached it, or `null` |
| `loops` | Solver loop iterations until the start is reached or the flood stops |
| `tokens` | The number of tokens. This is one token for each cell |
| `token_passes` | The number of token evaluations, for all loops |
| `open_cells` | The number of cells that are not walls |
| `reached_cells` | The number of cells that the search reached |
| `micros` | The solve time in microseconds. It does not include HTTP or JSON time |
| `heap.allocs` | The number of heap allocations during the solve |
| `heap.bytes` | The number of heap bytes allocated during the solve |
| `heap.peak` | The largest number of heap bytes live at one time during the solve, above the level at the start |

## Errors

| Condition | Body |
|-----------|------|
| Side out of range | `width must be between 5 and 321` |
| Wrong number of cells | `invalid maze: board has 225 cells, got 224` |
| Unknown character | `invalid maze: unexpected cell 'x'` |
| No start, or two starts | `invalid maze: maze needs exactly one start` |
| No goal | `invalid maze: maze needs at least one goal` |


## Example

```bash
curl -s localhost:3000/api/generate \
  -H 'content-type: application/json' \
  -d '{"shape":"square","algorithm":"prim","width":9,"height":9,"seed":1}'
```
