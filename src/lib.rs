//! Maze solving with a fixed-rule model compiled from breadth-first search.
//!
//! [`Model::trace`] applies the rules directly to maze cells. [`Model::solve`]
//! uses staged BFS frontiers, with a direct cached path for a single-goal,
//! fully open square grid. [`Transformer`] instead evaluates the fixed weights
//! of the looped graph transformer on every cell. [`WallSkippingTransformer`]
//! evaluates the same weights on traversable cells only. All three implement
//! [`Solver`] for graphs with at most [`MAX_DEGREE`] neighbours per cell.

mod maze;
mod model;
mod transformer;

pub use maze::{Cell, MAX_DEGREE, Maze, MazeError};
pub use model::{Model, Trace};
pub use transformer::{Transformer, WallSkippingTransformer};

/// Finds shortest-path legs using the maze's neighbour order to break ties.
pub trait Solver {
    /// Visits the nearest unvisited goal on each leg, or returns `None` if any is unreachable.
    fn solve(&self, maze: &Maze) -> Option<Vec<usize>>;
}

impl Solver for Model {
    fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        Model::solve(self, maze)
    }
}
