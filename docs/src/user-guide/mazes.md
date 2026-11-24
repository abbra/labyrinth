# Mazes

A `Maze` is a graph. Each node is a cell. Each edge joins two cells that
share a side. The `Maze` type does not store geometry. Thus the solver
works with any tiling.

## Cell types

| `Cell` | Text | Meaning |
|--------|------|---------|
| `Wall` | `#` | You cannot go through this cell |
| `Open` | `.` or space | You can go through this cell |
| `Start` | `S` | The path starts here. Exactly one per maze |
| `Goal` | `G` | The path ends here. Exactly one per maze |

## Rules

A valid maze obeys these rules:

- Each edge names two cells that exist.
- An edge does not join a cell to itself.
- Each pair of cells has a maximum of one edge.
- Each cell has a maximum of `MAX_DEGREE` (8) neighbours.
- The maze has exactly one start and exactly one goal.

`Maze::new` and `str::parse` check these rules. They return a `MazeError`
when a rule is broken. See [Error Messages](../reference/errors.md).

## Build a square grid from text

Use `str::parse`. Write one row per line. All rows must have the same
width. The parser ignores blank lines.

```rust
# extern crate labyrinth;
use labyrinth::Maze;

let maze: Maze = "\
S..#
.#..
...G".parse()?;
# Ok::<(), labyrinth::MazeError>(())
```

The cell id of row `r`, column `c` is `r * width + c`. The neighbour order
of each cell is west, north, east, south. The parser includes walls in the
graph. Walls stay closed because the model never enters them.

## Build a maze of any shape

Use `Maze::new`. Give the cells in id order. Give each edge one time, as a
pair of ids. The order of the pair is not important.

This example is a ring of four cells:

```rust
# extern crate labyrinth;
use labyrinth::{Cell, Maze, Model};

let cells = vec![Cell::Start, Cell::Open, Cell::Goal, Cell::Wall];
let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
let maze = Maze::new(cells, edges)?;
assert_eq!(Model::new().solve(&maze), Some(vec![0, 1, 2]));
# Ok::<(), labyrinth::MazeError>(())
```

Each cell keeps its neighbours in the order that its edges appear. This
order is important only for ties. When two neighbours are at the same
distance from the goal, the path goes through the neighbour that comes
first.

## Read a maze

| Method | Returns |
|--------|---------|
| `cells()` | All cells, indexed by cell id |
| `neighbours(id)` | The ids of the cells next to `id`. Panics if `id` is out of range |
| `start()` | The id of the start cell |
| `goal()` | The id of the goal cell |
