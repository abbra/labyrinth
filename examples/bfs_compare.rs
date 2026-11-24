use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::hint::black_box;
use std::time::{Duration, Instant};

use labyrinth::{Cell, Maze, Model};

const SIDE: usize = 512;
const RUNS: usize = 9;

fn main() {
    let maze = open_square();
    let start = maze.start();
    let goal = maze.goal();
    let model = Model::new();
    let mut times: [Vec<Duration>; 4] = std::array::from_fn(|_| Vec::with_capacity(RUNS));

    for run in 0..RUNS {
        for offset in 0..4 {
            let solver = (run + offset) % 4;
            let (elapsed, path) = match solver {
                0 => time_model(&model, &maze),
                1 => time_queue_bfs(&maze),
                2 => time_bidirectional_bfs(&maze),
                3 => time_a_star(&maze),
                _ => unreachable!(),
            };
            check_path(&maze, &path, start, goal);
            times[solver].push(elapsed);
        }
    }

    let [
        mut model_times,
        mut bfs_times,
        mut bidirectional_times,
        mut a_star_times,
    ] = times;
    let model_ms = median_ms(&mut model_times);
    let bfs_ms = median_ms(&mut bfs_times);
    let bidirectional_ms = median_ms(&mut bidirectional_times);
    let a_star_ms = median_ms(&mut a_star_times);
    println!("workload=512x512_open_square_single_goal runs={RUNS}");
    println!("model_solve_ms={model_ms:.3}");
    println!("queue_bfs_ms={bfs_ms:.3}");
    println!("bidirectional_bfs_ms={bidirectional_ms:.3}");
    println!("a_star_ms={a_star_ms:.3}");
    println!("model_over_queue_bfs={:.2}x", model_ms / bfs_ms);
    println!(
        "model_over_bidirectional_bfs={:.2}x",
        model_ms / bidirectional_ms
    );
    println!("model_over_a_star={:.2}x", model_ms / a_star_ms);
}

fn time_model(model: &Model, maze: &Maze) -> (Duration, Vec<usize>) {
    let start = Instant::now();
    let path = black_box(model.solve(black_box(maze))).expect("open square is solvable");
    (start.elapsed(), path)
}

fn time_queue_bfs(maze: &Maze) -> (Duration, Vec<usize>) {
    let start = Instant::now();
    let path = black_box(queue_bfs(black_box(maze))).expect("open square is solvable");
    (start.elapsed(), path)
}

fn time_bidirectional_bfs(maze: &Maze) -> (Duration, Vec<usize>) {
    let start = Instant::now();
    let path = black_box(bidirectional_bfs(black_box(maze))).expect("open square is solvable");
    (start.elapsed(), path)
}

fn time_a_star(maze: &Maze) -> (Duration, Vec<usize>) {
    let start = Instant::now();
    let path = black_box(a_star(black_box(maze))).expect("open square is solvable");
    (start.elapsed(), path)
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

fn a_star(maze: &Maze) -> Option<Vec<usize>> {
    let start = maze.start();
    let goal = maze.goal();
    let mut distance = vec![usize::MAX; maze.cells().len()];
    let mut previous = vec![usize::MAX; maze.cells().len()];
    let mut frontier = BinaryHeap::new();
    distance[start] = 0;
    previous[start] = start;
    frontier.push(Reverse((square_manhattan(start, goal), 0, start)));

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
                    next_cost + square_manhattan(neighbour, goal),
                    next_cost,
                    neighbour,
                )));
            }
        }
    }
    path_from_previous(start, goal, &previous)
}

fn square_manhattan(a: usize, b: usize) -> usize {
    (a / SIDE).abs_diff(b / SIDE) + (a % SIDE).abs_diff(b % SIDE)
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

fn check_path(maze: &Maze, path: &[usize], start: usize, goal: usize) {
    assert_eq!(path.first(), Some(&start));
    assert_eq!(path.last(), Some(&goal));
    assert_eq!(path.len(), 2 * (SIDE - 1) + 1);
    assert!(path.iter().all(|&cell| maze.cells()[cell] != Cell::Wall));
    assert!(
        path.windows(2)
            .all(|pair| maze.neighbours(pair[0]).contains(&pair[1]))
    );
}

fn median_ms(times: &mut [Duration]) -> f64 {
    times.sort_unstable();
    times[times.len() / 2].as_secs_f64() * 1_000.0
}

fn open_square() -> Maze {
    let mut cells = vec![Cell::Open; SIDE * SIDE];
    cells[0] = Cell::Start;
    cells[SIDE * SIDE - 1] = Cell::Goal;

    let mut edges = Vec::with_capacity(2 * SIDE * (SIDE - 1));
    for row in 0..SIDE {
        for col in 0..SIDE {
            let cell = row * SIDE + col;
            if col + 1 < SIDE {
                edges.push((cell, cell + 1));
            }
            if row + 1 < SIDE {
                edges.push((cell, cell + SIDE));
            }
        }
    }
    Maze::new(cells, edges).expect("512x512 square is a valid maze")
}
