# Quick Start

## Solve a maze from Rust

1. Write the maze as text. Use `#` for a wall, `.` for an open cell, `S` for
   the start and `G` for the goal.
2. Parse the text into a `Maze`.
3. Make a `Model`.
4. Call `solve`.

```rust
# extern crate labyrinth;
use labyrinth::{Maze, Model};

let maze: Maze = "S.#\n#.#\n#.G".parse()?;
let path = Model::new().solve(&maze).expect("maze is solvable");
assert_eq!(path, [0, 1, 4, 7, 8]);
# Ok::<(), labyrinth::MazeError>(())
```

The path is a list of cell ids. It starts at the start cell and ends at the
goal cell. In a text maze, the id of row `r`, column `c` is
`r * width + c`.

`solve` returns `None` when no path exists.

## Solve a maze from a file

The `solve` example reads a maze from standard input. It marks the path
with `*`:

```bash
cargo run -q --release --example solve < maze.txt
```

## Start the web UI

1. Start the server:

   ```bash
   cargo run --release -p labyrinth-server
   ```

2. Open <http://127.0.0.1:3000/> in a browser.
3. Select a shape and a generator. Click **Generate maze**.
4. Click **Run transformer**.

To use a different address, set `LABYRINTH_ADDR`:

```bash
LABYRINTH_ADDR=0.0.0.0:8080 cargo run --release -p labyrinth-server
```

## Next steps

- [Mazes](../user-guide/mazes.md) tells how to build mazes of any shape.
- [Web UI](../user-guide/web-ui.md) tells how to edit and solve mazes in
  the browser.
