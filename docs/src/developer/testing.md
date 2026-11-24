# Testing

Run all tests in release mode. Debug mode is slow on the large tests.

```bash
cargo test -q --release --workspace
```

## Library tests

`tests/solve.rs` compares the model with a reference BFS.

| Test | What it checks |
|------|----------------|
| `matches_bfs_on_random_square_mazes` | Path length and validity on random square mazes |
| `matches_bfs_on_hex_triangle_and_irregular_mazes` | The same checks on hex, triangle and random graphs |
| `walls_stay_walls_at_max_degree` | A wall with 8 reached neighbours does not become reached |
| `long_corridor_maze` | Many loops on a long corridor |

The doctest in `src/model.rs` also runs. The Rust examples in this book do
not run with `cargo test`. See [Check the book examples](#check-the-book-examples).

## Server tests

| Test | What it checks |
|------|----------------|
| `tilings_have_expected_degrees` | The maximum number of neighbours for each shape |
| `penrose_cells_are_unit_rhombs` | Each P3 cell is a rhombus with unit edges |
| `kites_and_darts_have_edges_one_and_phi_and_darts_are_concave` | Each P2 edge is 1 or φ. About 1/φ² of the cells are concave |
| `carved_mazes_are_solvable_on_every_shape` | Each generator makes a solvable maze on each shape |
| `same_seed_same_maze` | Generation is repeatable |
| `large_boards_are_carved_throughout` | At 321, more than one third of the cells are open |

## Rules for new tests

- Compare with BFS. Do not compare with a fixed path, because ties can
  have more than one shortest path.
- Use a fixed seed. Tests must give the same result each time.
- Do a test of the behaviour that a user sees, not of the internal steps.

## Check the book examples

The Rust examples in this book are tested by `mdbook test`:

```bash
cargo build --release
mdbook test docs -L target/release
```
