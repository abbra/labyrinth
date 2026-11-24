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

use labyrinth::{Cell, Maze, Model};

const SWEEP_SIDES: [usize; 5] = [64, 128, 256, 512, 1024];
const SEED: u64 = 1;
const RUNS: usize = 9;

fn main() {
    for side in SWEEP_SIDES {
        let open = open_square(side);
        benchmark(
            &format!("open_square_{side}x{side}_single_goal"),
            &open,
            side,
        );

        // Square generation only reads these fields; skip expensive polygon geometry.
        let board = board::Board {
            shape: board::Shape::Square,
            width: side,
            height: side,
            polygons: Vec::new(),
            neighbours: Vec::new(),
        };
        let cells = generate::generate(&board, generate::Algorithm::Backtracker, SEED);
        let maze = square_maze(side, &cells);
        benchmark(
            &format!("square_backtracker_{side}x{side}_seed_{SEED}"),
            &maze,
            side,
        );
    }
}

fn benchmark(name: &str, maze: &Maze, width: usize) {
    let start = maze.start();
    let goal = maze.goal();
    let shortest = queue_bfs(maze).expect("benchmark maze is solvable");
    let shortest_length = shortest.len();
    check_path(maze, &shortest, start, goal, shortest_length);

    let model = Model::new();
    let mut times: [Vec<Duration>; 4] = std::array::from_fn(|_| Vec::with_capacity(RUNS));
    for run in 0..RUNS {
        for offset in 0..4 {
            let solver = (run + offset) % 4;
            let (elapsed, path) = match solver {
                0 => time_model(&model, maze),
                1 => time_queue_bfs(maze),
                2 => time_bidirectional_bfs(maze),
                3 => time_a_star(maze, width),
                _ => unreachable!(),
            };
            check_path(maze, &path, start, goal, shortest_length);
            times[solver].push(elapsed);
        }
    }

    let [
        mut model_times,
        mut bfs_times,
        mut bidirectional_times,
        mut a_star_times,
    ] = times;
    let model_us = median_us(&mut model_times);
    let bfs_us = median_us(&mut bfs_times);
    let bidirectional_us = median_us(&mut bidirectional_times);
    let a_star_us = median_us(&mut a_star_times);
    println!("workload={name} cells={} runs={RUNS}", maze.cells().len());
    println!("model_solve_us={model_us:.1}");
    println!("queue_bfs_us={bfs_us:.1}");
    println!("bidirectional_bfs_us={bidirectional_us:.1}");
    println!("a_star_us={a_star_us:.1}");
}

fn time_model(model: &Model, maze: &Maze) -> (Duration, Vec<usize>) {
    timing::time_batch(|| model.solve(black_box(maze)))
}

fn time_queue_bfs(maze: &Maze) -> (Duration, Vec<usize>) {
    timing::time_batch(|| queue_bfs(black_box(maze)))
}

fn time_bidirectional_bfs(maze: &Maze) -> (Duration, Vec<usize>) {
    timing::time_batch(|| bidirectional_bfs(black_box(maze)))
}

fn time_a_star(maze: &Maze, width: usize) -> (Duration, Vec<usize>) {
    timing::time_batch(|| a_star(black_box(maze), width))
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

fn bidirectional_bfs(maze: &Maze) -> Option<Vec<usize>> {
    let start = maze.start();
    let goal = maze.goal();
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

fn a_star(maze: &Maze, width: usize) -> Option<Vec<usize>> {
    let start = maze.start();
    let goal = maze.goal();
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
