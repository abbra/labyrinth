# Introduction

**Labyrinth** finds paths through a maze with a transformer. The model is not
trained on examples. `Model::new` generates its parameters from fixed solver
rules: attention heads copy reached-neighbour state, and MLP units implement
the BFS update rules. The generated parameters are the same for every maze.

This is weight-space programming: the rules are encoded in the transformer's
weights, then run through attention, an MLP and a residual stream.

## What the model does

- It accepts a maze of any size.
- It accepts any tiling: squares, hexagons, triangles, Penrose rhombs,
  Penrose kites and darts. The limit is 8 neighbours per cell.
- It visits every goal, connecting each leg with a shortest path.
- It returns no route when any goal is unreachable.
- It uses the same 1,472 weights for all mazes.

## Crates

| Crate | Purpose |
|-------|---------|
| **labyrinth** | Library. Maze graph, the rule-generated model, the solver loop |
| **labyrinth-server** | Web server. Board shapes, maze generators, HTTP API, browser UI |
| **labyrinth-wasm** | Browser bindings. Local generation and solving for the WASM demo |

The library has no dependencies. The server uses tokio, axum and serde. The
WASM bindings use wasm-bindgen and serde.

## Design goals

1. **Correctness.** Each leg is shortest; the full route visits all goals. Tests
   compare single-goal results with a reference BFS.
2. **Fixed weights.** The weights do not change with the maze size or the
   tiling.
3. **Exact values.** Each loop ends with features that are exactly 0 or 1.
   Small errors cannot grow from loop to loop.
4. **Speed.** A 321×321 maze solves in about 0.3 s on 24 cores. See
   [Performance Results](reference/performance.md).

## Background

These sources describe weight-space programming:

- [LoopedWSP](https://mbzuai-oryx.github.io/LoopedWSP/)
- [Sparks of weight-space programming](https://rikiyat.github.io/blog/weight-space-programming/)
- [Tracr: compiled transformers](https://arxiv.org/abs/2301.05062)
- [arXiv 2510.08734](https://arxiv.org/abs/2510.08734)

## Next step

Read [Installation](getting-started/installation.md).
