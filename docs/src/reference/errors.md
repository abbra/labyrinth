# Error Messages

## MazeError

`Maze::new` and `str::parse` return these errors. `MazeError` is
`#[non_exhaustive]`. Thus a `match` must have a `_` arm.

| Variant | Message | Cause |
|---------|---------|-------|
| `Empty` | `maze has no rows` | The text has no rows that are not blank |
| `Ragged { row }` | `row {row} has a different width than row 0` | A text row has a different length |
| `BadChar { ch, row, col }` | `unexpected {ch:?} at row {row}, column {col}` | A character is not `#`, `.`, space, `S` or `G` |
| `UnknownCell { edge }` | `edge {edge:?} names an unknown cell` | An edge has an id that is not less than the number of cells |
| `SelfLoop { cell }` | `cell {cell} is its own neighbour` | An edge joins a cell to itself |
| `DuplicateEdge { edge }` | `edge {edge:?} appears twice` | Two edges join the same pair of cells |
| `TooManyNeighbours { cell }` | `cell {cell} has more than 8 neighbours` | A cell has more than `MAX_DEGREE` edges |
| `Start` | `maze needs exactly one start` | The maze has no start, or more than one |
| `Goal` | `maze needs exactly one goal` | The maze has no goal, or more than one |

Row and column numbers start at 0.

## Model panics

These panics show a defect in the weights. Report them as bugs.

| Message | Cause |
|---------|-------|
| `no fixed point after {n} loops` | The loop did not stop after `cells + 1` loops |
| `pointer chain broke off after {path:?}` | A cell on the path has no pointer |

## Server errors

See [HTTP API: Errors](../user-guide/http-api.md#errors).
