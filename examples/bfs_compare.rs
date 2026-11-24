use std::collections::VecDeque;
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
    let mut model_times = Vec::with_capacity(RUNS);
    let mut bfs_times = Vec::with_capacity(RUNS);

    for run in 0..RUNS {
        if run % 2 == 0 {
            let (elapsed, path) = time_model(&model, &maze);
            check_path(&maze, &path, start, goal);
            model_times.push(elapsed);
            let (elapsed, path) = time_queue_bfs(&maze);
            check_path(&maze, &path, start, goal);
            bfs_times.push(elapsed);
        } else {
            let (elapsed, path) = time_queue_bfs(&maze);
            check_path(&maze, &path, start, goal);
            bfs_times.push(elapsed);
            let (elapsed, path) = time_model(&model, &maze);
            check_path(&maze, &path, start, goal);
            model_times.push(elapsed);
        }
    }

    let model_ms = median_ms(&mut model_times);
    let bfs_ms = median_ms(&mut bfs_times);
    println!("workload=512x512_open_square_single_goal runs={RUNS}");
    println!("model_solve_ms={model_ms:.3}");
    println!("queue_bfs_ms={bfs_ms:.3}");
    println!("model_over_queue_bfs={:.2}x", model_ms / bfs_ms);
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
    if previous[goal] == usize::MAX {
        return None;
    }

    let mut path = vec![goal];
    while *path.last().unwrap() != start {
        path.push(previous[*path.last().unwrap()]);
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
