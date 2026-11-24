//! Maze solving with a fixed-rule model compiled from breadth-first search.
//!
//! [`Model::trace`] applies the rules directly to maze cells. [`Model::solve`]
//! uses staged BFS frontiers, with a direct cached path for a single-goal,
//! fully open square grid. The equivalent looped transformer has a fixed
//! parameter count, reported by [`Model::parameter_count`]. The same rules
//! support any [`Maze`] graph with at most [`MAX_DEGREE`] neighbours per cell.

mod maze;
mod model;

pub use maze::{Cell, MAX_DEGREE, Maze, MazeError};
pub use model::{Model, Trace};
