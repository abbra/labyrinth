# Introduction

**Labyrinth** finds paths through mazes with fixed breadth-first-search rules.
`Model::new` creates a zero-sized solver; it does not construct transformer
weights. The solver applies the rules directly to maze cells.

The solver is equivalent to a looped transformer with 1,472 fixed parameters.
`parameter_count()` reports that equivalent parameter count; the runtime does
not execute attention, an MLP or a residual stream.

## What the model does

- It accepts mazes of any size that fit in memory.
- It accepts any tiling with at most 8 neighbours per cell: squares, hexagons,
  triangles, Penrose rhombs, kites and darts.
- It visits every goal, joining goals with shortest-path legs.
- It returns no route when any goal is unreachable.
- Its equivalent transformer has the same 1,472 parameters for every maze.

## Crates

| Crate | Purpose |
|-------|---------|
| **labyrinth** | Library. Maze graph, fixed-rule solver and solver loop |
| **labyrinth-server** | Web server. Board shapes, maze generators, HTTP API, browser UI |
| **labyrinth-wasm** | Browser bindings. Local generation and solving for the WASM demo |

The library has no dependencies. The server uses tokio, axum and serde. The
WASM bindings use wasm-bindgen and serde.

## Design goals

1. **Correctness.** Each route leg is shortest; the full route visits all goals.
   Tests compare single-goal results with a reference BFS.
2. **Maze-independent rules.** The same rules work for every maze size and
   tiling.
3. **Discrete state.** The runtime uses integer state, not floating-point
   activations.
4. **Speed.** The solver reevaluates only cells affected by changes. See
   [Performance Results](reference/performance.md).

## Background

These sources describe weight-space programming:

- [LoopedWSP](https://mbzuai-oryx.github.io/LoopedWSP/)
- [Sparks of weight-space programming](https://rikiyat.github.io/blog/weight-space-programming/)
- [Tracr: compiled transformers](https://arxiv.org/abs/2301.05062)
- [arXiv 2510.08734](https://arxiv.org/abs/2510.08734)

## Next step

Read [Installation](getting-started/installation.md).
