# Solving

## Make the model

```rust
# extern crate labyrinth;
use labyrinth::Model;

let model = Model::new();
```

`Model::new` writes all weights. The weights do not depend on a maze.
Make one model and use it for all mazes. `Model` is `Clone`, `Send` and
`Sync`. The web server shares one model between all requests.

`model.parameter_count()` returns the number of weights. The value is
1,472.

## Get the path

`solve` returns the cell ids of a shortest path:

```rust
# extern crate labyrinth;
# use labyrinth::{Maze, Model};
# let maze: Maze = "S.G".parse()?;
match Model::new().solve(&maze) {
    Some(path) => println!("{} steps", path.len() - 1),
    None => println!("no path"),
}
# Ok::<(), labyrinth::MazeError>(())
```

- The first id is the start. The last id is the goal.
- Two ids that follow each other are neighbours.
- No other path is shorter.
- `None` means that the goal is unreachable from the start.

## Get statistics

`trace` does the same work as `solve`. It also returns a `Trace`:

| Field | Meaning |
|-------|---------|
| `path` | The same value as `solve` |
| `loops` | The number of times the transformer block ran |
| `token_passes` | The number of token evaluations, for all loops. The solver skips tokens that cannot change |
| `reached_at` | For each cell: the loop in which the search from the goal reached it, or `None` |

`loops` is equal to the path length when a path exists. It is equal to the
depth of the search when no path exists.

Use `reached_at` to show the search as an animation. The web UI does this.

## Threads

`trace` starts one worker thread for each CPU. The threads stop before
`trace` returns. A loop with fewer than 32 changed tokens runs on the
calling thread only.

## Panics

`solve` and `trace` panic in these conditions:

- The loop does not stop after `cells + 1` loops.
- The chain of pointers from the start does not reach the goal.

Each condition is a defect in the weights. A correct model never panics.
The panic message identifies the condition. See
[Error Messages](../reference/errors.md).

A panic in a worker thread goes to the calling thread. It does not cause
a hang.
