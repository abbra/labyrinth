# Introduction

**Labyrinth** finds shortest paths with a fixed-rule BFS equivalent to a
looped transformer. `Model::new` creates a zero-sized solver; it does not
construct transformer weights. `Model::trace` applies the rules directly and
records search progress. `Model::solve` uses staged BFS frontiers for general
mazes, and a cached direct path for a single-goal, fully open square grid.

`Transformer` runs the numerical reference model: eight attention heads and
a ReLU MLP update each cell's residual stream. Its 1,472 weights are fixed
across mazes. Both `Model` and `Transformer` implement `Solver`.

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
| **labyrinth** | Library. Maze graph, optimized BFS solver, trace and transformer |
| **labyrinth-server** | Web server. Board shapes, maze generators, HTTP API, browser UI |
| **labyrinth-wasm** | Browser bindings. Local generation and solving for the WASM demo |

The library has no dependencies. The server uses tokio, axum and serde. The
WASM bindings use wasm-bindgen and serde.

## Design goals

1. **Correctness.** Each route leg is shortest; the full route visits all goals.
   Tests compare single-goal results with a reference BFS.
2. **Maze-independent rules.** The same rules work for every maze size and
   tiling.
3. **Discrete state.** The optimized `Model` uses integer state instead of
   the transformer's floating-point activations.
4. **Speed.** General mazes use staged BFS frontiers; `trace` evaluates only
   dirty cells. The single-goal open-square `solve` path avoids the flood.
   See [Performance Results](reference/performance.md).

## Background

These sources describe weight-space programming:

- [LoopedWSP](https://mbzuai-oryx.github.io/LoopedWSP/)
- [Sparks of weight-space programming](https://rikiyat.github.io/blog/weight-space-programming/)
- [Tracr: compiled transformers](https://arxiv.org/abs/2301.05062)
- [arXiv 2510.08734](https://arxiv.org/abs/2510.08734)

## Next step

Read [Installation](getting-started/installation.md).
