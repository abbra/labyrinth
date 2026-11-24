# Contributing

## Building

```bash
cargo build --release --workspace
```

## Code style

- `cargo fmt --all` sets the format.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass with
  no warnings.
- Keep weight construction deterministic and based on the solver rules. Do
  not train maze-specific weights.

## Before you send a change

Do these steps in this order:

1. `cargo fmt --all`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -q --release --workspace`

## Commits

- Make one logical change in each commit.
- Write the subject line in the imperative mood. Keep it shorter than 72
  characters.

## Documentation

The book uses [mdbook](https://rust-lang.github.io/mdBook/) and
[mdbook-mermaid](https://github.com/badboy/mdbook-mermaid).

```bash
cargo install mdbook mdbook-mermaid
mdbook build docs/
mdbook serve docs/    # preview at http://localhost:3000
```

`mdbook serve` uses port 3000 by default. The labyrinth server uses the
same port. Use `mdbook serve docs/ -p 3001` when the server is running.

Write the book in simplified technical language:

- Use short sentences. Write one instruction in each sentence.
- Use the active voice. Use the imperative for procedures.
- Use one word for one meaning. For example, always write "cell", not
  "tile" or "square" for a maze position.
- Use tables for values, options and fields.
- Use numbered lists for procedures. Use bullet lists for other lists.

API documentation:

```bash
cargo doc --workspace --no-deps --open
```
