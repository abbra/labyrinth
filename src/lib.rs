//! Maze solving by a transformer whose weights are generated from fixed BFS rules.
//!
//! [`Model`] is a looped transformer: one attention + MLP block applied until the
//! residual stream reaches a fixed point. Its weights encode breadth-first search, so
//! the same weights find the shortest path in any [`Maze`]: a graph of cells from any
//! tiling (squares, hexagons, triangles, Penrose rhombs, ...) with at most
//! [`MAX_DEGREE`] neighbours per cell.

mod maze;
mod model;

pub use maze::{Cell, MAX_DEGREE, Maze, MazeError};
pub use model::{Model, Trace};
