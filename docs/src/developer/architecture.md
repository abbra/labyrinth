# Architecture Overview

Labyrinth is a Rust workspace with three crates.

```mermaid
graph LR
    LIB[labyrinth<br/>maze graph + model] --> SRV[labyrinth-server<br/>boards, generators, HTTP, UI]
    LIB --> WASM[labyrinth-wasm<br/>browser bindings]
    SRV --> UI[index.html<br/>shared browser UI]
    WASM --> UI
```

## labyrinth (library)

| File | Content |
|------|---------|
| `src/lib.rs` | Public exports |
| `src/maze.rs` | `Maze`, `Cell`, `MazeError`, `MAX_DEGREE`, text parser |
| `src/model.rs` | `Model`, `Trace`, staged BFS solver and fixed-rule trace loop |

The library has no geometry. A maze is cells plus edges. The neighbour
lists use compressed sparse row (CSR) storage: one `offsets` array and one
`targets` array.

### Solve and trace

`meet_in_middle` grows waves from the start and from every goal and rebuilds
the route to the first goal reached that `trace` would choose (see
[How the Model Works](../reference/model.md#single-goal-search)). With several
goals, `visit_remaining_goals` adds BFS legs to the remaining goals. For a
single-goal, fully open square grid, `Maze` caches directional tie-break
preferences and `solve` constructs a Manhattan-shortest path directly. No
`solve` strategy runs the trace loop.

`Model::trace` always runs the fixed-rule dirty-cell loop:

```mermaid
sequenceDiagram
    participant C as Calling thread
    C->>C: initialize cell state and mark all cells dirty
    loop until start reached or no dirty cells
        C->>C: apply BFS rules to dirty cells
        C->>C: write changed state and mark those cells and neighbours dirty
    end
    C->>C: build route through every goal
```

`trace` evaluates only dirty cells sequentially; both library methods are
sequential and create no worker threads or transformer weights.

## labyrinth-server

| File | Content |
|------|---------|
| `server/src/main.rs` | Routes, request and response types, side limits |
| `server/src/board.rs` | `Shape`, `Board`, the tilings, the shared-edge neighbour search |
| `server/src/generate.rs` | `Algorithm`, the generators, start and goal placement |
| `server/src/index.html` | The UI: HTML, CSS and JavaScript in one file |

## labyrinth-wasm

`wasm/src/lib.rs` exposes maze generation and solving through `wasm-bindgen`.
It compiles the same board and generator modules used by the server. The
`wasm-demo/` entry point opens the shared UI with its browser-only backend.

The server binary includes `index.html` at compile time. Thus the server has no
files to install.

### Request flow

```mermaid
sequenceDiagram
    participant B as Browser
    participant S as Server
    participant M as Model
    B->>S: POST /api/generate
    S->>S: Board::new, generate
    S-->>B: cells + polygons
    B->>B: rasterize, draw
    B->>S: POST /api/solve (edited cells)
    S->>S: Board::new, Board::maze
    S->>M: trace (blocking thread)
    M-->>S: Trace
    S-->>B: path + statistics
```

The server does not keep state between requests. It makes the board
again for each solve.

The server accepts concurrent clients. It runs up to two solves at once.
Each solve runs on a blocking thread and holds a slot until it finishes.
Maze generation also runs off the async workers. The heap counters are
process-wide, so overlapping solves can affect each other's heap statistics.

## Design decisions

| Decision | Reason |
|----------|--------|
| Shared fixed rules | Work for all maze sizes and supported tilings |
| Neighbour-slot ordering | Resolves ties deterministically |
| Discrete state updates | Avoid floating-point drift |
| Panic on broken solver invariants | A wrong path is worse than a crash |
| Geometry only in the server | The library stays small and general |
| Pixel raster of cell ids in the UI | Drawing and clicks are fast on large boards |
