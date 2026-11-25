//! A fixed-rule breadth-first-search solver with route tracing.
//!
//! `Model::solve` uses staged BFS frontiers for general graphs and a cached
//! direct path for a single-goal, fully open square grid. With one goal it also
//! grows waves from the start, always widening the smaller frontier, and rebuilds
//! the route `trace` would choose. `Model::trace` applies boolean BFS rules
//! directly to dirty cells, floods outward from all goals, and records how each
//! cell was reached. Both return shortest-path legs joining every goal.
//!
//! The equivalent graph transformer uses structural attention: each head selects one
//! neighbour slot, with a sink for missing slots. Fixed weights therefore work for any
//! maze size and any tiling with at most [`crate::maze::MAX_DEGREE`] neighbours per cell.
use std::collections::VecDeque;

use crate::maze::{Cell, MAX_DEGREE as K, Maze};

const MODEL_D: usize = 4 + 2 * K;
type State = u8;
// Stage one wave without letting its rule outputs affect other candidates.
const PENDING: State = 1 << 7;
// Start-side cells: this flag plus their depth from the start, modulo 3, in bits 0-1.
const FORWARD: State = 1 << 6;
// Start-side cells reached from more than one cell of the previous wave.
const TIED: State = PENDING;
// Start-side cells that lie on a shortest route to the goal.
const ON_ROUTE: State = 1 << 5;
// Start-side bits that identify a cell's wave and route mark, without the tie flag.
const WAVE: State = FORWARD | ON_ROUTE | 3;
const _: () = assert!(K + 1 < ON_ROUTE as usize);

/// The fixed-rule maze-solving model.
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
    /// Panics if the model fails to terminate, which would indicate a bug in the solver rules.
    pub fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        if let Some((side, preference)) = maze.open_square() {
            return Some(open_square_path(maze, side, preference));
        }
        let start = maze.start();
        let mut x = embed(maze);
        let mut frontier = Vec::with_capacity(maze.goals().len());
        for &goal in maze.goals() {
            x[goal] = self.block(goal, &x, maze);
            frontier.push(goal);
        }
        if maze.goals().len() == 1 {
            return meet_in_middle(maze, x, frontier);
        }
        let mut candidates = Vec::new();
        while x[start] == 0 && !frontier.is_empty() {
            grow_goal_wave(maze, &mut x, &mut frontier, &mut candidates);
        }
        read_path(maze, &x)
    }

    /// Returns the route produced by [`Model::solve`] and details of how the
    /// search reached each cell.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would indicate a bug in the solver rules.
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
                for &j in maze.neighbours(i) {
                    if !std::mem::replace(&mut marked[j], true) {
                        dirty.push(j);
                    }
                }
                if !std::mem::replace(&mut marked[i], true) {
                    dirty.push(i);
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

    /// Applies one fixed-rule BFS update to a cell.
    fn block(&self, i: usize, x: &[State], maze: &Maze) -> State {
        let cell = maze.cells()[i];
        if cell == Cell::Goal {
            return (K + 1) as State;
        }
        if cell == Cell::Wall || x[i] != 0 {
            return x[i];
        }
        for (slot, &neighbour) in maze.neighbours(i).iter().enumerate() {
            if x[neighbour] != 0 && x[neighbour] & PENDING == 0 {
                return slot as State + 1;
            }
        }
        0
    }
}

fn open_square_path(maze: &Maze, side: usize, preferences: &[u8]) -> Vec<usize> {
    let goal = maze.goal();
    let (goal_row, goal_col) = (goal / side, goal % side);
    let mut cell = maze.start();
    let (mut row, mut col) = (cell / side, cell % side);
    let distance = row.abs_diff(goal_row) + col.abs_diff(goal_col);
    let mut path = Vec::with_capacity(distance + 1);
    path.push(cell);
    let bit = usize::from(row < goal_row) * 2 + usize::from(col < goal_col);
    let preference_mask = 1u8 << bit;
    while cell != goal {
        let vertical = row != goal_row;
        let horizontal = col != goal_col;
        let vertical_first = if vertical && horizontal {
            preferences[cell] & preference_mask != 0
        } else {
            vertical
        };
        if vertical_first {
            if row < goal_row {
                row += 1;
                cell += side;
            } else {
                row -= 1;
                cell -= side;
            }
        } else if col < goal_col {
            col += 1;
            cell += 1;
        } else {
            col -= 1;
            cell -= 1;
        }
        path.push(cell);
    }
    path
}

/// Whether the goal flood has settled `state` (a pointer slot or the goal itself).
fn goal_side(state: State) -> bool {
    state != 0 && state < ON_ROUTE
}

/// Grows the goal flood by one wave; stops early and returns `true` on touching a
/// start-side cell.
fn grow_goal_wave(
    maze: &Maze,
    x: &mut [State],
    frontier: &mut Vec<usize>,
    candidates: &mut Vec<usize>,
) -> bool {
    candidates.clear();
    for &i in frontier.iter() {
        for &j in maze.neighbours(i) {
            if x[j] & FORWARD != 0 {
                return true;
            }
            if x[j] == 0 && maze.cells()[j] != Cell::Wall {
                let slot = maze
                    .neighbours(j)
                    .iter()
                    .position(|&neighbour| goal_side(x[neighbour]))
                    .expect("a candidate has a reached neighbour");
                x[j] = (slot + 1) as State | PENDING;
                candidates.push(j);
            }
        }
    }
    for &j in candidates.iter() {
        x[j] &= !PENDING;
    }
    std::mem::swap(frontier, candidates);
    false
}

/// Single-goal search growing whole waves from both ends, smaller frontier first.
///
/// Goal-side pointers match the plain goal flood on every route cell. For start-side
/// cells the route takes the lowest slot whose neighbour is one step closer to the
/// goal, which is the neighbour one step further from the start that still lies on a
/// shortest route. When one cell meets the goal side and no cell back to the start was
/// reached twice, that route follows the recorded parents; otherwise route cells are
/// marked level by level, walking back from the meeting cells, and walked forward.
fn meet_in_middle(maze: &Maze, mut x: Vec<State>, mut back: Vec<usize>) -> Option<Vec<usize>> {
    // Parent indices into `order` are stored as `u32`.
    assert!(u32::try_from(x.len()).is_ok(), "maze too large");
    let start = maze.start();
    x[start] = FORWARD;
    // Start-side cells in wave order, with the `order` index of the cell that first
    // reached each; level `t` begins at `levels[t]`.
    let mut order = vec![start];
    let mut parents = vec![0u32];
    let mut levels = vec![0];
    let mut candidates = Vec::new();
    let mut goal_waves = 0;
    'search: loop {
        let depth = levels.len() - 1;
        let (level_start, level_end) = (levels[depth], order.len());
        if level_start == level_end || back.is_empty() {
            return None;
        }
        if back.len() < level_end - level_start {
            if grow_goal_wave(maze, &mut x, &mut back, &mut candidates) {
                break;
            }
            goal_waves += 1;
            continue;
        }
        let tag = FORWARD | ((depth + 1) % 3) as State;
        for i in level_start..level_end {
            for &j in maze.neighbours(order[i]) {
                let state = x[j];
                if goal_side(state) {
                    order.truncate(level_end);
                    break 'search;
                }
                if state == 0 && maze.cells()[j] != Cell::Wall {
                    x[j] = tag;
                    order.push(j);
                    parents.push(i as u32);
                } else if state & WAVE == tag {
                    x[j] |= TIED;
                }
            }
        }
        levels.push(level_end);
    }

    // The waves touched between the last start-side level and settled goal-side cells.
    let last = levels.len() - 1;
    let mut route = candidates;
    route.clear();
    let mut meeting = 0;
    for (k, &cell) in order.iter().enumerate().skip(levels[last]) {
        if maze.neighbours(cell).iter().any(|&j| goal_side(x[j])) {
            meeting = k;
            route.push(cell);
        }
    }
    // The goal side adds at most one cell per goal wave plus the meeting cell.
    let capacity = last + goal_waves + 2;
    let chain = match route[..] {
        [_] => parent_chain(&x, &order, &parents, meeting, last, capacity),
        _ => None,
    };
    let mut path = match chain {
        Some(path) => path,
        None => {
            for &cell in &route {
                x[cell] |= ON_ROUTE;
            }
            // Neighbours one level nearer the start carry the previous depth tag, so only
            // cells next to the route are touched rather than every start-side cell.
            let mut nearer = back;
            for depth in (1..=last).rev() {
                let tag = FORWARD | ((depth - 1) % 3) as State;
                nearer.clear();
                for &cell in &route {
                    for &j in maze.neighbours(cell) {
                        if x[j] & WAVE == tag {
                            x[j] |= ON_ROUTE;
                            nearer.push(j);
                        }
                    }
                }
                std::mem::swap(&mut route, &mut nearer);
            }
            let mut cell = start;
            let mut path = Vec::with_capacity(capacity);
            path.push(cell);
            for depth in 1..=last {
                let tag = FORWARD | ON_ROUTE | (depth % 3) as State;
                cell = *maze
                    .neighbours(cell)
                    .iter()
                    .find(|&&j| x[j] & WAVE == tag)
                    .unwrap_or_else(|| panic!("route broke off after {path:?}"));
                path.push(cell);
            }
            path
        }
    };
    let mut cell = *maze
        .neighbours(path[last])
        .iter()
        .find(|&&j| goal_side(x[j]))
        .unwrap_or_else(|| panic!("route broke off after {path:?}"));
    path.push(cell);
    while cell != maze.goal() {
        cell = follow_pointer(maze, &x, cell, &path);
        path.push(cell);
    }
    Some(path)
}

/// The start-side route ending at `order[k]`, `depth` steps from the start, following
/// recorded parents; `None` if a cell on it was reached twice, so another parent might
/// be the one [`Model::trace`] picks.
fn parent_chain(
    x: &[State],
    order: &[usize],
    parents: &[u32],
    mut k: usize,
    depth: usize,
    capacity: usize,
) -> Option<Vec<usize>> {
    let mut path = Vec::with_capacity(capacity);
    path.resize(depth + 1, 0);
    for step in (1..=depth).rev() {
        let cell = order[k];
        if x[cell] & TIED != 0 {
            return None;
        }
        path[step] = cell;
        k = parents[k] as usize;
    }
    path[0] = order[k];
    Some(path)
}

/// The neighbour a goal-side pointer at `cell` leads to.
fn follow_pointer(maze: &Maze, x: &[State], cell: usize, path: &[usize]) -> usize {
    let slot = x[cell]
        .checked_sub(1)
        .map(usize::from)
        .filter(|&slot| slot < K)
        .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
    maze.neighbours(cell)[slot]
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
            cell = follow_pointer(maze, x, cell, &path);
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
        cell = follow_pointer(maze, x, cell, &path);
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
