# Installation

## Requirements

- Rust 1.85 or later. The crates use edition 2024.
- A web browser, if you use the web UI.
- `mdbook` and `mdbook-mermaid`, if you build this book.

## Get the source

```bash
git clone <repository-url> labyrinth
cd labyrinth
```

## Build

Build the library and the server:

```bash
cargo build --release --workspace
```

Always use `--release` for real mazes. A debug build is much slower.

## Build the browser-only demo

Install Fedora's `rust-std-static-wasm32-unknown-unknown` package and use a
`wasm-bindgen` CLI matching the version pinned in `Cargo.lock` (0.2.127).
`wasm-pack` is not required.

```bash
cargo build --release -p labyrinth-wasm --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir wasm/pkg target/wasm32-unknown-unknown/release/labyrinth_wasm.wasm
```

Serve the repository root over HTTP and open `/wasm-demo/`. Maze generation and
solving then run in the browser; heap allocation statistics are not available.

## Use the library in your crate

Add a path dependency to your `Cargo.toml`:

```toml
[dependencies]
labyrinth = { path = "../labyrinth" }
```

The crate is not on crates.io.

## Make sure that the build works

Run the example on a small maze:

```bash
printf 'S.#\n#.#\n#.G\n' | cargo run -q --release --example solve
```

The output must be:

```text
S*#
#*#
#*G
4 steps, 1472 weights
```

## Next step

Read [Quick Start](quickstart.md).
