//! Solver dispatch shared by the HTTP server and the WASM bindings.
//!
//! Wraps the library solvers and the classical reference searches from the
//! `bfs_compare` harness. Every search breaks ties by the maze's neighbour order, so
//! routes match the model contract. The harness keeps allocation-free copies of these
//! searches for timing; keep the two in sync.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use labyrinth::{Cell, Maze, Model, Solver, Transformer, WallSkippingTransformer};

/// The runnable solver implementations, in the paper's comparison order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Model,
    Bfs,
    BidirectionalBfs,
    AStar,
    Transformer,
    WallSkipping,
    GhScanBase,
}

/// Wire names, also the UI dropdown order.
pub const METHODS: [(Method, &str); 7] = [
    (Method::Model, "model"),
    (Method::Bfs, "bfs"),
    (Method::BidirectionalBfs, "bidirectional_bfs"),
    (Method::AStar, "a_star"),
    (Method::Transformer, "transformer"),
    (Method::WallSkipping, "wall_skipping"),
    (Method::GhScanBase, "gh_scan_base"),
];

/// Parses a wire name.
pub fn parse_method(value: &str) -> Option<Method> {
    METHODS
        .iter()
        .find(|(_, name)| *name == value)
        .map(|(method, _)| *method)
}

/// The error for an unknown wire name, listing the accepted ones.
pub fn unknown_method(value: &str) -> String {
    let names = METHODS
        .iter()
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(", ");
    format!("unknown solver: {value} (expected one of {names})")
}

/// What a solver run observed.
#[derive(Debug)]
pub struct Outcome {
    /// Route from start through every goal, or `None` when a goal is unreachable.
    pub path: Option<Vec<usize>>,
    /// Loop, wave or distance at which each cell was first reached; empty for the
    /// dense transformers, which keep no per-cell search state.
    pub reached_at: Vec<Option<usize>>,
    /// Model loops, or the largest search wave or distance.
    pub loops: usize,
    /// Model only: token evaluations summed over loops.
    pub token_passes: usize,
    /// Cells the search reached; every open cell for the dense transformers.
    pub reached_cells: usize,
}

/// Runs the selected method. `square_width` is the width of square boards, `None`
/// otherwise.
pub fn solve(maze: &Maze, square_width: Option<usize>, method: Method) -> Result<Outcome, String> {
    match method {
        Method::Model => {
            let trace = Model::new().trace(maze);
            let reached_cells = trace.reached_at.iter().flatten().count();
            Ok(Outcome {
                path: trace.path,
                reached_at: trace.reached_at,
                loops: trace.loops,
                token_passes: trace.token_passes,
                reached_cells,
            })
        }
        Method::Bfs => Ok(bfs(maze)),
        Method::BidirectionalBfs => Ok(legs(maze, |wave, from, goal| {
            bidirectional_bfs(maze, wave, from, goal)
        })),
        Method::AStar => {
            let Some(width) = square_width else {
                return Err("a_star needs a square board for its Manhattan heuristic".into());
            };
            Ok(legs(maze, |wave, from, goal| {
                a_star(maze, width, wave, from, goal)
            }))
        }
        Method::Transformer => Ok(dense(Transformer::new().solve(maze), maze)),
        Method::WallSkipping => Ok(dense(WallSkippingTransformer::new().solve(maze), maze)),
        Method::GhScanBase => Ok(gh_scan_base(maze)),
    }
}

fn dense(path: Option<Vec<usize>>, maze: &Maze) -> Outcome {
    Outcome {
        path,
        reached_at: Vec::new(),
        loops: 0,
        token_passes: 0,
        reached_cells: maze
            .cells()
            .iter()
            .filter(|&&cell| cell != Cell::Wall)
            .count(),
    }
}

/// Records the first wave that reached `cell`; later legs never overwrite it.
fn note(wave: &mut [Option<usize>], cell: usize, depth: usize) {
    if wave[cell].is_none() {
        wave[cell] = Some(depth);
    }
}

fn finish(wave: Vec<Option<usize>>, path: Option<Vec<usize>>) -> Outcome {
    Outcome {
        loops: wave.iter().flatten().max().copied().unwrap_or(0),
        reached_cells: wave.iter().flatten().count(),
        path,
        reached_at: wave,
        token_passes: 0,
    }
}

/// Queue BFS legs, each to the nearest unvisited goal: the route `Model::solve` returns.
fn bfs(maze: &Maze) -> Outcome {
    let n = maze.cells().len();
    let mut unvisited = vec![false; n];
    for &goal in maze.goals() {
        unvisited[goal] = true;
    }
    let mut remaining = maze.goals().len();
    let mut previous = vec![usize::MAX; n];
    let mut depth = vec![usize::MAX; n];
    let mut wave = vec![None; n];
    let mut queue = VecDeque::new();
    let mut leg = Vec::new();
    let mut path = vec![maze.start()];
    let route = 'route: {
        while remaining > 0 {
            let from = *path.last().expect("every leg starts at the route's end");
            previous.fill(usize::MAX);
            depth.fill(usize::MAX);
            previous[from] = from;
            depth[from] = 0;
            queue.clear();
            queue.push_back(from);
            let mut target = None;
            while let Some(cell) = queue.pop_front() {
                if unvisited[cell] {
                    target = Some(cell);
                    break;
                }
                for &neighbour in maze.neighbours(cell) {
                    if maze.cells()[neighbour] != Cell::Wall && previous[neighbour] == usize::MAX {
                        previous[neighbour] = cell;
                        depth[neighbour] = depth[cell] + 1;
                        queue.push_back(neighbour);
                    }
                }
            }
            for (slot, &d) in wave.iter_mut().zip(&depth) {
                if d != usize::MAX && slot.is_none() {
                    *slot = Some(d);
                }
            }
            let Some(mut cell) = target else {
                break 'route None;
            };
            leg.clear();
            while cell != from {
                leg.push(cell);
                cell = previous[cell];
            }
            for &cell in leg.iter().rev() {
                if std::mem::take(&mut unvisited[cell]) {
                    remaining -= 1;
                }
                path.push(cell);
            }
        }
        break 'route Some(path);
    };
    finish(wave, route)
}

/// Runs point-to-point searches, always to the nearest unvisited goal by the maze's
/// neighbour order; each search merges its own per-cell waves.
fn legs(
    maze: &Maze,
    mut search: impl FnMut(&mut Vec<Option<usize>>, usize, usize) -> Option<Vec<usize>>,
) -> Outcome {
    let mut wave = vec![None; maze.cells().len()];
    let start = maze.start();
    let route = 'route: {
        if let &[goal] = maze.goals() {
            break 'route search(&mut wave, start, goal);
        }
        let mut remaining = maze.goals().to_vec();
        let mut from = start;
        let mut route = Vec::new();
        while !remaining.is_empty() {
            let mut chosen = 0;
            let Some(mut leg) = search(&mut wave, from, remaining[0]) else {
                break 'route None;
            };
            for (i, &goal) in remaining.iter().enumerate().skip(1) {
                let Some(candidate) = search(&mut wave, from, goal) else {
                    break 'route None;
                };
                if precedes(maze, &candidate, &leg) {
                    chosen = i;
                    leg = candidate;
                }
            }
            from = remaining.swap_remove(chosen);
            if route.is_empty() {
                route = leg;
            } else {
                route.extend_from_slice(&leg[1..]);
            }
        }
        break 'route Some(route);
    };
    finish(wave, route)
}

/// Whether route `a` precedes `b` in BFS visit order: shorter, or on a tie the earlier
/// neighbour's subtree first at every depth.
fn precedes(maze: &Maze, a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return a.len() < b.len();
    }
    for (i, (&left, &right)) in a.iter().zip(b).enumerate().skip(1) {
        if left != right {
            let neighbours = maze.neighbours(a[i - 1]);
            return neighbours.iter().find(|&&n| n == left || n == right) == Some(&left);
        }
    }
    false
}

/// Bidirectional BFS between two cells, meeting in the middle layer by layer.
fn bidirectional_bfs(
    maze: &Maze,
    wave: &mut [Option<usize>],
    start: usize,
    goal: usize,
) -> Option<Vec<usize>> {
    let mut from_start = vec![usize::MAX; maze.cells().len()];
    let mut from_goal = vec![usize::MAX; maze.cells().len()];
    let mut previous = vec![usize::MAX; maze.cells().len()];
    let mut next = vec![usize::MAX; maze.cells().len()];
    let mut start_frontier = VecDeque::new();
    let mut goal_frontier = VecDeque::new();
    from_start[start] = 0;
    from_goal[goal] = 0;
    previous[start] = start;
    next[goal] = goal;
    start_frontier.push_back(start);
    goal_frontier.push_back(goal);
    let mut start_depth = 0;
    let mut goal_depth = 0;
    let mut best_distance = usize::MAX;
    let mut meeting = usize::MAX;

    while !start_frontier.is_empty() && !goal_frontier.is_empty() {
        if meeting != usize::MAX && start_depth + goal_depth >= best_distance {
            break;
        }
        if start_frontier.len() <= goal_frontier.len() {
            let layer_len = start_frontier.len();
            for _ in 0..layer_len {
                let cell = start_frontier
                    .pop_front()
                    .expect("frontier layer is populated");
                for &neighbour in maze.neighbours(cell) {
                    if maze.cells()[neighbour] == Cell::Wall || from_start[neighbour] != usize::MAX
                    {
                        continue;
                    }
                    from_start[neighbour] = start_depth + 1;
                    previous[neighbour] = cell;
                    start_frontier.push_back(neighbour);
                    if from_goal[neighbour] != usize::MAX {
                        let distance = from_start[neighbour] + from_goal[neighbour];
                        if distance < best_distance {
                            best_distance = distance;
                            meeting = neighbour;
                        }
                    }
                }
            }
            start_depth += 1;
        } else {
            let layer_len = goal_frontier.len();
            for _ in 0..layer_len {
                let cell = goal_frontier
                    .pop_front()
                    .expect("frontier layer is populated");
                for &neighbour in maze.neighbours(cell) {
                    if maze.cells()[neighbour] == Cell::Wall || from_goal[neighbour] != usize::MAX {
                        continue;
                    }
                    from_goal[neighbour] = goal_depth + 1;
                    next[neighbour] = cell;
                    goal_frontier.push_back(neighbour);
                    if from_start[neighbour] != usize::MAX {
                        let distance = from_start[neighbour] + from_goal[neighbour];
                        if distance < best_distance {
                            best_distance = distance;
                            meeting = neighbour;
                        }
                    }
                }
            }
            goal_depth += 1;
        }
    }
    for (cell, (&from, &to)) in from_start.iter().zip(&from_goal).enumerate() {
        let depth = from.min(to);
        if depth != usize::MAX {
            note(wave, cell, depth);
        }
    }
    if meeting == usize::MAX {
        return None;
    }

    let mut path = Vec::with_capacity(best_distance + 1);
    let mut cell = meeting;
    while cell != start {
        path.push(cell);
        cell = previous[cell];
    }
    path.push(start);
    path.reverse();
    cell = meeting;
    while cell != goal {
        cell = next[cell];
        path.push(cell);
    }
    Some(path)
}

/// A* with the square Manhattan heuristic between two cells.
fn a_star(
    maze: &Maze,
    width: usize,
    wave: &mut [Option<usize>],
    start: usize,
    goal: usize,
) -> Option<Vec<usize>> {
    let mut distance = vec![usize::MAX; maze.cells().len()];
    let mut previous = vec![usize::MAX; maze.cells().len()];
    let mut frontier = BinaryHeap::new();
    distance[start] = 0;
    previous[start] = start;
    frontier.push(Reverse((square_manhattan(start, goal, width), 0, start)));

    while let Some(Reverse((_, cost, cell))) = frontier.pop() {
        if cost != distance[cell] {
            continue;
        }
        if cell == goal {
            break;
        }
        for &neighbour in maze.neighbours(cell) {
            if maze.cells()[neighbour] == Cell::Wall {
                continue;
            }
            let next_cost = cost + 1;
            if next_cost < distance[neighbour] {
                distance[neighbour] = next_cost;
                previous[neighbour] = cell;
                frontier.push(Reverse((
                    next_cost + square_manhattan(neighbour, goal, width),
                    next_cost,
                    neighbour,
                )));
            }
        }
    }
    for (cell, &d) in distance.iter().enumerate() {
        if d != usize::MAX {
            note(wave, cell, d);
        }
    }
    path_from_previous(start, goal, &previous)
}

fn square_manhattan(a: usize, b: usize, width: usize) -> usize {
    (a / width).abs_diff(b / width) + (a % width).abs_diff(b % width)
}

fn path_from_previous(start: usize, goal: usize, previous: &[usize]) -> Option<Vec<usize>> {
    if previous[goal] == usize::MAX {
        return None;
    }
    let mut path = Vec::new();
    let mut cell = goal;
    loop {
        path.push(cell);
        if cell == start {
            break;
        }
        cell = previous[cell];
    }
    path.reverse();
    Some(path)
}

/// Gholami–Hajiaghayi Algorithm 8 base case: Dijkstra by scanning the pending labels
/// for the minimum, then a tight-edge tree per leg.
fn gh_scan_base(maze: &Maze) -> Outcome {
    let mut wave = vec![None; maze.cells().len()];
    let mut unvisited = vec![false; maze.cells().len()];
    for &goal in maze.goals() {
        unvisited[goal] = true;
    }
    let mut route = vec![maze.start()];
    let mut source = maze.start();
    let route = 'route: {
        for _ in maze.goals() {
            let distance = source_distances(maze, source);
            for (cell, &d) in distance.iter().enumerate() {
                if d != usize::MAX {
                    note(&mut wave, cell, d);
                }
            }
            let mut parent = vec![usize::MAX; distance.len()];
            parent[source] = source;
            let mut queue = VecDeque::from([source]);
            let mut target = None;
            while let Some(cell) = queue.pop_front() {
                if target.is_none() && unvisited[cell] {
                    target = Some(cell);
                }
                // Finish the tight-edge tree even after finding the leg's target.
                for &next in maze.neighbours(cell) {
                    if parent[next] == usize::MAX && distance[next] == distance[cell] + 1 {
                        parent[next] = cell;
                        queue.push_back(next);
                    }
                }
            }
            let Some(goal) = target else {
                break 'route None;
            };
            let leg = route.len();
            let mut cell = goal;
            while cell != source {
                route.push(cell);
                cell = parent[cell];
            }
            route[leg..].reverse();
            unvisited[goal] = false;
            source = goal;
        }
        break 'route Some(route);
    };
    finish(wave, route)
}

fn source_distances(maze: &Maze, source: usize) -> Vec<usize> {
    let mut distance = vec![usize::MAX; maze.cells().len()];
    distance[source] = 0;
    let mut pending = vec![source];
    while !pending.is_empty() {
        // Scan finite unsettled labels for the minimum; no heap or FIFO ordering.
        let min = (0..pending.len())
            .min_by_key(|&i| distance[pending[i]])
            .expect("pending is non-empty");
        let cell = pending.swap_remove(min);
        let offer = distance[cell] + 1;
        for &next in maze.neighbours(cell) {
            if maze.cells()[next] != Cell::Wall && offer < distance[next] {
                if distance[next] == usize::MAX {
                    pending.push(next);
                }
                distance[next] = offer;
            }
        }
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use labyrinth::Cell::{Goal, Open, Start};

    fn square(side: usize, cells: &[u8]) -> Maze {
        assert_eq!(cells.len(), side * side);
        let mut edges = Vec::new();
        for cell in 0..side * side {
            if cell % side + 1 < side {
                edges.push((cell, cell + 1));
            }
            if cell / side + 1 < side {
                edges.push((cell, cell + side));
            }
        }
        Maze::new(
            cells
                .iter()
                .map(|&cell| match cell {
                    b'#' => Cell::Wall,
                    b'.' => Cell::Open,
                    b'S' => Cell::Start,
                    b'G' => Cell::Goal,
                    _ => panic!("unexpected cell {}", cell as char),
                })
                .collect(),
            edges,
        )
        .unwrap()
    }

    #[test]
    fn tied_goals_follow_neighbour_order_at_the_fork() {
        let maze = Maze::new(vec![Start, Open, Goal, Goal], [(0, 1), (1, 3), (1, 2)]).unwrap();
        assert_eq!(
            legs(&maze, |wave, from, goal| bidirectional_bfs(
                &maze, wave, from, goal
            ))
            .path,
            Some(vec![0, 1, 3, 1, 2])
        );
        assert_eq!(
            legs(&maze, |wave, from, goal| a_star(&maze, 2, wave, from, goal)).path,
            Some(vec![0, 1, 3, 1, 2])
        );
        assert_eq!(bfs(&maze).path, Some(vec![0, 1, 3, 1, 2]));
        assert_eq!(gh_scan_base(&maze).path, Some(vec![0, 1, 3, 1, 2]));
    }

    #[test]
    fn scans_match_queue_bfs_on_tied_goals() {
        let maze = square(3, b"G...S...G");
        assert_eq!(gh_scan_base(&maze).path, bfs(&maze).path);
        // 0 and 2 are not adjacent, so the leg back to goal 8 revisits cell 1.
        assert_eq!(bfs(&maze).path, Some(vec![4, 1, 0, 1, 2, 5, 8]));
    }

    #[test]
    fn unreachable_goals_and_geometry_errors() {
        let maze = square(3, b"SG.###..G");
        assert_eq!(bfs(&maze).path, None);
        assert_eq!(gh_scan_base(&maze).path, None);
        assert!(solve(&maze, None, Method::AStar).is_err());
        assert_eq!(solve(&maze, Some(3), Method::AStar).unwrap().path, None);
        assert!(parse_method("spiral").is_none());
    }
}
