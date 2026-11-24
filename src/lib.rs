//! Maze solving by a transformer whose weights are programmed by hand, not trained.
//!
//! [`Model`] is a looped transformer: one attention + MLP block applied until the
//! residual stream reaches a fixed point. Its weights encode breadth-first search, so
//! the same weights find the shortest path in any [`Maze`].

mod maze;
mod model;

pub use maze::{Cell, Maze, ParseMazeError, Pos};
pub use model::Model;
