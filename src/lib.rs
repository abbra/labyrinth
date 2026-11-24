//! Maze solving with a fixed-rule model compiled from breadth-first search.
//!
//! [`Model`] applies the rules directly to maze cells. The equivalent looped
//! transformer has a fixed parameter count, reported by
//! [`Model::parameter_count`]. The same rules solve any [`Maze`] graph with at
//! most [`MAX_DEGREE`] neighbours per cell.

mod maze;
mod model;

pub use maze::{Cell, MAX_DEGREE, Maze, MazeError};
pub use model::{Model, Trace};
