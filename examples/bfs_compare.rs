#[allow(dead_code)]
#[path = "../server/src/board.rs"]
mod board;
#[allow(dead_code)]
#[path = "../server/src/generate.rs"]
mod generate;
mod timing;

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::hint::black_box;
use std::time::Duration;

#[path = "bfs_compare/gh_base.rs"]
mod gh_base;
#[path = "bfs_compare/nearest.rs"]
mod nearest;
#[path = "bfs_compare/options.rs"]
mod options;

use labyrinth::{Cell, Maze, Model, Solver, Transformer, WallSkippingTransformer};

const METHODS: [&str; 7] = [
    "model",
    "bfs",
    "bidirectional_bfs",
    "a_star",
    "transformer",
    "wall_skipping",
    "gh_scan_base",
];

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "--help") {
        println!("{}", options::HELP);
        return;
    }
    let opts = options::parse(args).unwrap_or_else(|error| {
        eprintln!("{error}\n{}", options::HELP);
        std::process::exit(2)
    });
    let selected = |name| opts.workload.as_deref().is_none_or(|w| w == name);
    for side in opts.sizes {
        if selected("open") {
            benchmark("open", side, None, opts.runs, &open_square(side));
        }
        if !selected("backtracker") && !selected("multi") {
            continue;
        }
        let board = square_board(side);
        for seed in 1..=opts.seeds {
            let mut cells =
                generate::generate(&board, generate::Algorithm::Backtracker, seed as u64);
            if selected("backtracker") {
                benchmark(
                    "backtracker",
                    side,
                    Some(seed),
                    opts.runs,
                    &square_maze(side, &cells),
                );
            }
            if selected("multi") {
                let (far, mid) = ((side - 3) | 1, (side / 2) | 1);
                for (row, col) in [(1, far), (far, 1), (mid, mid)] {
                    cells[row * side + col] = b'G';
                }
                benchmark(
                    "multi",
                    side,
                    Some(seed),
                    opts.runs,
                    &square_maze(side, &cells),
                );
            }
        }
    }
}

fn square_board(side: usize) -> board::Board {
    // Square generation only reads the dimensions; skip expensive polygon geometry.
    board::Board {
        shape: board::Shape::Square,
        width: side,
        height: side,
        polygons: Vec::new(),
        neighbours: Vec::new(),
    }
}

fn benchmark(name: &str, side: usize, seed: Option<usize>, runs: usize, maze: &Maze) {
    let transformer = Transformer::new();
    let compact = WallSkippingTransformer::new();
    let model = Model::new();
    let bfs = if maze.goals().len() == 1 {
        queue_bfs
    } else {
        nearest_goal_bfs
    };
    let expected = bfs(maze).expect("benchmark maze is solvable");
    println!(
        "workload={name} side={side} seed={} goals={} steps={} runs={runs}",
        seed.map_or_else(|| "-".to_owned(), |s| s.to_string()),
        maze.goals().len(),
        expected.len() - 1
    );
    let mut times: [_; METHODS.len()] = std::array::from_fn(|_| Vec::with_capacity(runs));
    for run in 0..runs {
        for offset in 0..METHODS.len() {
            let method = (run + offset) % METHODS.len();
            let (elapsed, path) = timing::time_batch(|| {
                let maze = black_box(maze);
                match method {
                    0 => model.solve(maze),
                    1 => bfs(maze),
                    2 => nearest::visit(maze, |s, g| bidirectional_bfs(maze, s, g)),
                    3 => nearest::visit(maze, |s, g| a_star(maze, side, s, g)),
                    4 => transformer.solve(maze),
                    5 => compact.solve(maze),
                    6 => gh_base::solve(maze),
                    _ => unreachable!(),
                }
            });
            if matches!(method, 2 | 3) && maze.goals().len() == 1 {
                // Point-to-point baselines may choose another equally short path.
                check_path(maze, &path, maze.start(), maze.goal(), expected.len());
            } else {
                assert_eq!(path, expected, "{name} side {side}, {}", METHODS[method]);
            }
            times[method].push(elapsed);
        }
    }
    for (method, mut samples) in METHODS.into_iter().zip(times) {
        println!("{method}_us={:.3}", median_us(&mut samples));
    }
}

fn queue_bfs(maze: &Maze) -> Option<Vec<usize>> {
    let start = maze.start();
    let goal = maze.goal();
    let mut previous = vec![usize::MAX; maze.cells().len()];
    let mut queue = VecDeque::new();
    previous[start] = start;
    queue.push_back(start);

    while let Some(cell) = queue.pop_front() {
        if cell == goal {
            break;
        }
        for &neighbour in maze.neighbours(cell) {
            if maze.cells()[neighbour] != Cell::Wall && previous[neighbour] == usize::MAX {
                previous[neighbour] = cell;
                queue.push_back(neighbour);
            }
        }
    }
    path_from_previous(start, goal, &previous)
}

/// Queue BFS legs, each to the nearest unvisited goal: the multi-goal route `solve` returns.
fn nearest_goal_bfs(maze: &Maze) -> Option<Vec<usize>> {
    let n = maze.cells().len();
    let mut unvisited = vec![false; n];
    for &goal in maze.goals() {
        unvisited[goal] = true;
    }
    let mut remaining = maze.goals().len();
    let mut previous = vec![usize::MAX; n];
    let mut queue = VecDeque::new();
    let mut leg = Vec::new();
    let mut path = vec![maze.start()];
    while remaining > 0 {
        let from = *path.last().unwrap();
        previous.fill(usize::MAX);
        previous[from] = from;
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
                    queue.push_back(neighbour);
                }
            }
        }
        let mut cell = target?;
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
    Some(path)
}

fn bidirectional_bfs(maze: &Maze, start: usize, goal: usize) -> Option<Vec<usize>> {
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

fn a_star(maze: &Maze, width: usize, start: usize, goal: usize) -> Option<Vec<usize>> {
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

fn check_path(maze: &Maze, path: &[usize], start: usize, goal: usize, shortest: usize) {
    assert_eq!(path.first(), Some(&start));
    assert_eq!(path.last(), Some(&goal));
    assert_eq!(path.len(), shortest);
    assert!(path.iter().all(|&cell| maze.cells()[cell] != Cell::Wall));
    assert!(
        path.windows(2)
            .all(|pair| maze.neighbours(pair[0]).contains(&pair[1]))
    );
}

fn median_us(times: &mut [Duration]) -> f64 {
    times.sort_unstable();
    times[times.len() / 2].as_secs_f64() * 1_000_000.0
}

fn open_square(side: usize) -> Maze {
    let mut cells = vec![Cell::Open; side * side];
    cells[0] = Cell::Start;
    cells[side * side - 1] = Cell::Goal;
    Maze::new(cells, square_edges(side)).expect("open square is a valid maze")
}

fn square_maze(side: usize, cells: &[u8]) -> Maze {
    assert_eq!(cells.len(), side * side);
    let cells = cells
        .iter()
        .map(|&cell| match cell {
            b'#' => Cell::Wall,
            b'.' => Cell::Open,
            b'S' => Cell::Start,
            b'G' => Cell::Goal,
            _ => panic!("unexpected generated cell {}", cell as char),
        })
        .collect();
    Maze::new(cells, square_edges(side)).expect("generated square is a valid maze")
}

fn square_edges(side: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity(2 * side * (side - 1));
    for cell in 0..side * side {
        if cell % side + 1 < side {
            edges.push((cell, cell + 1));
        }
        if cell / side + 1 < side {
            edges.push((cell, cell + side));
        }
    }
    edges
}
