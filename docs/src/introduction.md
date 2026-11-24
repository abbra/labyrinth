# Introduction

**Labyrinth** finds paths through a maze with a transformer.
Nobody trained this transformer. Each weight has a value that a person
wrote by hand. The weights implement breadth-first search (BFS).

This approach is known as weight-space programming. You write the program
directly into the weights of the network. Then you run the network in the
usual way: attention, MLP, residual stream.

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
| **labyrinth** | Library. Maze graph, the hand-written model, the solver loop |
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
