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

`solve` returns a route from the start through every goal:
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

- The first id is the start; the last id is a goal.
- Consecutive ids are neighbouring cells.
- Each leg reaches the nearest unvisited goal by a shortest path; the route may revisit cells.
- `None` means at least one goal cannot be reached by the route.

## Get statistics

`trace` does the same work as `solve`. It also returns a `Trace`:

| Field | Meaning |
|-------|---------|
| `path` | The same value as `solve` |
| `loops` | The number of times the transformer block ran |
| `token_passes` | The number of token evaluations, for all loops. The solver skips tokens that cannot change |
| `reached_at` | For each cell: the loop in which the search from the goals reached it, or `None` |

`loops` measures the multi-source transformer flood, not the length of the full
multi-goal route. It stops when the start is reached or the flood makes no progress.

Use `reached_at` to show the search as an animation. The web UI does this.

## Threads

`trace` starts one worker thread for each CPU. The threads stop before
`trace` returns. A loop with fewer than 32 changed tokens runs on the
calling thread only.

## Panics

`solve` and `trace` panic in these conditions:

- The chain of pointers from the start does not reach the first goal.

Each condition is a defect in the weights. A correct model never panics.
The panic message identifies the condition. See
[Error Messages](../reference/errors.md).

A panic in a worker thread goes to the calling thread. It does not cause
a hang.
