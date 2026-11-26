# Labyrinth

A maze solver in Rust built on weight-space programming: a breadth-first
search equivalent to a looped transformer, written as maze-independent rules.
One set of fixed rules solves any maze size and any tiling with at most eight
neighbours per cell — squares, hexagons, triangles, Penrose rhombs, kites and
darts — and visits every goal.

- **Documentation**: <https://abbra.github.io/labyrinth/>
- **Browser-only demo** (no server needed): <https://abbra.github.io/labyrinth/demo/wasm-demo/>

## Crates

| Crate | Purpose |
|-------|---------|
| **labyrinth** | Library. Maze graph, optimized BFS solver, trace and transformer |
| **labyrinth-server** | Web server. Board shapes, maze generators, HTTP API, browser UI |
| **labyrinth-wasm** | Browser bindings. Local generation and solving for the WASM demo |

## Quick start

Start the web UI (then open <http://127.0.0.1:3000/>):

```bash
cargo run --release -p labyrinth-server
```

Solve a maze from the command line; the path is marked `*` (`#` wall, `.`
open, `S` start, `G` goal):

```bash
printf 'S.#\n#.#\n#.G' | cargo run -q --release --example solve
```

Build the browser-only demo (wasm-bindgen 0.2.129) and run the test suite
with `scripts/local-ci.sh` — see
[Installation](https://abbra.github.io/labyrinth/getting-started/installation.html).

## Solvers

| Method | Idea |
|--------|------|
| `model` | Fixed-rule model solver, the default |
| `bfs` | Plain breadth-first baseline |
| `bidirectional_bfs` | Waves from the start and the goals meet in the middle |
| `a_star` | A* with a Manhattan heuristic |
| `transformer` | Numerical transformer reference (1,472 fixed weights) |
| `wall_skipping` | Skips over runs of straight walls |
| `gh_scan_base` | Scan-based baseline |

The HTTP API accepts the method as `?solver=`; the web UI offers the same
seven.

## Development

`scripts/local-ci.sh` runs the same jobs locally that CI runs on GitHub
(`.github/workflows/`): build, fmt, clippy, test, doc, examples, wasm,
wasm-dist. Merges to `main` rebuild the
[documentation](https://abbra.github.io/labyrinth/) and republish the demo.

Licensed under [GPL-3.0](COPYING).
