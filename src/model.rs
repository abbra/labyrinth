//! A looped solver compiled from the fixed-weight transformer's BFS rules.
//!
//! Each maze cell is a token. The fused update copies reached state from the first
//! reached neighbour and applies the transformer's boolean BFS rules directly. It floods
//! outward from all goals, storing a pointer to the neighbour that first reached each
//! cell. The solver then joins shortest paths to any remaining goals.
//!
//! The equivalent graph transformer uses structural attention: each head selects one
//! neighbour slot, with a sink for missing slots. Fixed weights therefore work for any
//! maze size and any tiling with at most [`crate::maze::MAX_DEGREE`] neighbours per cell.
use std::collections::VecDeque;

use crate::maze::{Cell, MAX_DEGREE as K, Maze};

const MODEL_D: usize = 4 + 2 * K;
type State = u8;

/// The compiled maze-solving transformer.
///
/// The update rules are fixed and independent of the maze.
///
/// # Examples
///
/// ```
/// use labyrinth::{Maze, Model};
/// let maze: Maze = "S.#\n#.#\n#.G".parse()?;
/// let path = Model::new().solve(&maze).expect("maze is solvable");
/// assert_eq!(path, [0, 1, 4, 7, 8]);
/// # Ok::<(), labyrinth::MazeError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Model;

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Creates the fixed-rule solver.
    pub fn new() -> Self {
        Self
    }

    /// Number of weights in the equivalent generated transformer, biases included.
    pub fn parameter_count(&self) -> usize {
        K * (K + 1 + 2 * MODEL_D) + (3 * K + 3) * 2 * MODEL_D
    }

    /// Returns a route from the start through every goal, or `None` when any goal is
    /// unreachable from the route.
    ///
    /// For multiple goals, it visits the nearest unvisited goal next, using a shortest
    /// path for each leg.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        self.trace(maze).path
    }

    /// Like [`Model::solve`], but also reports how the model got there.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn trace(&self, maze: &Maze) -> Trace {
        let mut x = embed(maze);
        let n = maze.cells().len();
        let mut reached_at = vec![None; n];
        let mut marked = vec![false; n];
        let mut dirty: Vec<_> = (0..n).collect();
        let mut updates = Vec::new();
        let (start, max_loops) = (maze.start(), n + 1);
        let (mut loops, mut token_passes) = (0, 0);
        loop {
            if x[start] != 0 || dirty.is_empty() {
                break;
            }
            loops += 1;
            assert!(loops <= max_loops, "no fixed point after {max_loops} loops");
            token_passes += dirty.len();
            for &i in &dirty {
                let token = self.block(i, &x, maze);
                if x[i] != token {
                    updates.push((i, token));
                }
            }
            dirty.clear();
            for (i, token) in updates.drain(..) {
                x[i] = token;
                reached_at[i].get_or_insert(loops);
                for j in std::iter::once(i).chain(maze.neighbours(i).iter().copied()) {
                    if !std::mem::replace(&mut marked[j], true) {
                        dirty.push(j);
                    }
                }
            }
            for &i in &dirty {
                marked[i] = false;
            }
        }
        Trace {
            path: read_path(maze, &x),
            loops,
            token_passes,
            reached_at,
        }
    }

    /// Applies one transformer's boolean BFS update to a cell.
    fn block(&self, i: usize, x: &[State], maze: &Maze) -> State {
        let cell = maze.cells()[i];
        if cell == Cell::Goal {
            return (K + 1) as State;
        }
        if cell == Cell::Wall || x[i] != 0 {
            return x[i];
        }
        maze.neighbours(i)
            .iter()
            .position(|&j| x[j] != 0)
            .map_or(0, |slot| slot as State + 1)
    }
}

/// What [`Model::trace`] observed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trace {
    /// Same as [`Model::solve`].
    pub path: Option<Vec<usize>>,
    /// Loop iterations until the start was reached or the flood stopped growing.
    pub loops: usize,
    /// Token evaluations by the block, summed over all loops; tokens whose
    /// neighbourhood did not change are not re-evaluated.
    pub token_passes: usize,
    /// Per cell id: the loop in which the flood fill from the goals reached it.
    pub reached_at: Vec<Option<usize>>,
}

/// Follows the first goal's pointer chain, then routes through any remaining goals.
fn read_path(maze: &Maze, x: &[State]) -> Option<Vec<usize>> {
    let mut cell = maze.start();
    if x[cell] == 0 {
        return None;
    }
    if maze.goals().len() == 1 {
        let mut path = vec![cell];
        while cell != maze.goal() {
            let slot = x[cell]
                .checked_sub(1)
                .map(usize::from)
                .filter(|&slot| slot < K)
                .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
            cell = maze.neighbours(cell)[slot];
            path.push(cell);
        }
        return Some(path);
    }

    let n = maze.cells().len();
    let mut unvisited = vec![false; n];
    for &goal in maze.goals() {
        unvisited[goal] = true;
    }
    let mut remaining = maze.goals().len();
    let mut previous = vec![usize::MAX; n];
    let mut queue = VecDeque::with_capacity(n);
    let mut segment = Vec::with_capacity(n);
    let mut path = vec![cell];
    while !unvisited[cell] {
        let slot = x[cell]
            .checked_sub(1)
            .map(usize::from)
            .filter(|&slot| slot < K)
            .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
        cell = maze.neighbours(cell)[slot];
        path.push(cell);
    }
    unvisited[cell] = false;
    remaining -= 1;
    while remaining > 0 {
        cell = *path.last().unwrap();
        previous.fill(usize::MAX);
        previous[cell] = cell;
        queue.clear();
        queue.push_back(cell);
        let mut target = None;
        while let Some(at) = queue.pop_front() {
            if unvisited[at] {
                target = Some(at);
                break;
            }
            for &next in maze.neighbours(at) {
                if maze.cells()[next] != Cell::Wall && previous[next] == usize::MAX {
                    previous[next] = at;
                    queue.push_back(next);
                }
            }
        }
        let mut at = target?;
        segment.clear();
        segment.push(at);
        while at != cell {
            at = previous[at];
            segment.push(at);
        }
        for &at in segment.iter().rev().skip(1) {
            if unvisited[at] {
                unvisited[at] = false;
                remaining -= 1;
            }
            path.push(at);
        }
    }
    Some(path)
}

/// Every cell starts unreached.
fn embed(maze: &Maze) -> Vec<State> {
    vec![0; maze.cells().len()]
}
