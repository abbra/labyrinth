# Tilings and Generators

The server makes each board as a list of cell outlines (polygons). Two
cells are neighbours when they share a full edge. The server finds shared
edges by matching vertices on a grid of 0.0001 units.

## Shapes

| Shape | API value | Cells | Neighbours per cell | Size means |
|-------|-----------|-------|---------------------|------------|
| Square | `square` | Squares | 4 | Cells across and down |
| Hexagon | `hex` | Pointy-top hexagons, odd rows shifted right | 6 | Cells across and down |
| Triangle | `triangle` | Up and down triangles | 3 | Cells across and down |
| Penrose rhombs (P3) | `penrose` | Thick and thin rhombs, unit edges | 4 | Rectangle size in unit edges |
| Penrose kites and darts (P2) | `kite-dart` | Kites and concave darts, edges 1 and φ | 4 | Rectangle size in unit edges |

The cell ids of square, hex and triangle boards are row-major. The cell
ids of the Penrose boards have no spatial order.

### Penrose tilings

The two Penrose tilings never repeat. The server makes them with Robinson
triangles:

1. Start with a wheel of ten golden triangles at the board centre.
2. Divide each triangle into smaller golden triangles. Do this until the
   edges are at unit scale.
3. After each step, remove the triangles that are far from the board.
4. Join each triangle to its mirror half to make one cell.
   - For rhombs, the halves join on the short side.
   - For kites and darts, the halves join on a long side.
5. Keep a cell if the centre of its join line is in the board rectangle.

Thus the board edge is not straight.

The darts are concave. About 1/φ² (38%) of the cells in a large
kite-and-dart board are darts.

## Generators

| Algorithm | Result | Paths |
|-----------|--------|-------|
| `backtracker` | Long corridors that turn many times | Exactly one |
| `prim` | Many short dead ends | Exactly one |
| `braid` | A backtracker maze with about 70% of its dead ends opened | Usually more than one |
| `caves` | Each cell is a wall with a 30% probability | Zero, one or more |

The same `seed` always gives the same maze.

### Square boards

Square boards use classic room carvers. Rooms are on odd rows and odd
columns. The carver opens the wall between two rooms.

### Other boards

The other shapes use a cell carver:

1. Open the cell that is nearest to the board centre.
2. Open a wall cell only if the passage that extends into it is its only
   open neighbour. This keeps corridors one cell wide.
3. Continue until no cell can be opened.

The carver starts at the centre because a corner cell can become closed in
after a few steps.

### Start and goal

The generator chooses the start and goal from the open cells at opposite
extremes of the positional score `x + y` (low for start, high for goal).
