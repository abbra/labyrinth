# Solving

## Make the model

```rust
# extern crate labyrinth;
use labyrinth::Model;

let model = Model::new();
```

`Model::new` creates the fixed-rule solver. It does not build parameters from
the maze or construct transformer weights. See
[How the Model Works](../reference/model.md) for the equivalent transformer
representation.
Make one model and use it for all mazes. `Model` is `Clone`, `Send` and
`Sync`. The web server shares one model between all requests.

`model.parameter_count()` returns the equivalent transformer's parameter
count, including biases. The value is 1,472.

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

`trace` returns the same route as `solve`, along with fixed-rule flood
statistics:

| Field | Meaning |
|-------|---------|
| `loops` | The number of flood iterations |

`loops` measures the multi-source goal flood, not the length of the full
multi-goal route. It stops when the start is reached or the flood makes no progress.

Use `reached_at` to show the search as an animation. The web UI does this.

## Execution

`Model::solve` and `Model::trace` are sequential. `solve` builds a direct
cached-order Manhattan path for a single-goal, fully open square grid, grows
waves from both ends for other single-goal mazes, and floods from the goals
with staged BFS frontiers when there are several goals; see
[How the Model Works](../reference/model.md#single-goal-search). `trace` always
evaluates dirty cells in
the fixed-rule loop. The web server runs each solve on a blocking thread and
allows at most two concurrent solves.

## Panics

`solve` and `trace` panic if the solver fails to terminate or if a
predecessor chain breaks while building the route. These indicate a solver
bug, not a maze validation error. See
[Error Messages](../reference/errors.md).
