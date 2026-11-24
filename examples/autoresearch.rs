use std::collections::VecDeque;
use std::hint::black_box;
use std::time::{Duration, Instant};

use labyrinth::{Cell, Maze, Model};

const SIDE: usize = 1024;
const RUNS: usize = 9;

fn main() {
    let maze = open_square();
    let model = Model::new();
    let expected_length = 2 * (SIDE - 1) + 1;
    let mut model_times = Vec::with_capacity(RUNS);
    let mut bfs_times = Vec::with_capacity(RUNS);

    for run in 0..RUNS {
        for offset in 0..2 {
            match (run + offset) % 2 {
                0 => {
                    let start = Instant::now();
                    let path = black_box(model.solve(black_box(&maze)))
                        .expect("open square must be solvable");
                    model_times.push(start.elapsed());
                    check_path(&maze, &path, expected_length);
                }
                _ => {
                    let start = Instant::now();
                    let path = black_box(queue_bfs(black_box(&maze)))
                        .expect("open square must be solvable");
                    bfs_times.push(start.elapsed());
                    check_path(&maze, &path, expected_length);
                }
            }
        }
    }

    let model_time = median(&mut model_times);
    let bfs_time = median(&mut bfs_times);
    println!(
        "METRIC speedup={:.6}",
        bfs_time.as_secs_f64() / model_time.as_secs_f64()
    );
    println!("METRIC solve_ms={:.3}", model_time.as_secs_f64() * 1_000.0);
    println!(
        "METRIC queue_bfs_ms={:.3}",
        bfs_time.as_secs_f64() * 1_000.0
    );
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
        for &neighbor in maze.neighbours(cell) {
            if previous[neighbor] == usize::MAX && maze.cells()[neighbor] != Cell::Wall {
                previous[neighbor] = cell;
                queue.push_back(neighbor);
            }
        }
    }

    (previous[goal] != usize::MAX).then(|| {
        let mut path = vec![goal];
        while *path.last().expect("path contains goal") != start {
            path.push(previous[*path.last().expect("path contains goal")]);
        }
        path.reverse();
        path
    })
}

fn check_path(maze: &Maze, path: &[usize], expected_length: usize) {
    assert_eq!(path.len(), expected_length);
    assert_eq!(path.first(), Some(&maze.start()));
    assert_eq!(path.last(), Some(&maze.goal()));
    assert!(path.iter().all(|&cell| maze.cells()[cell] != Cell::Wall));
    assert!(
        path.windows(2)
            .all(|pair| { maze.neighbours(pair[0]).contains(&pair[1]) })
    );
}

fn median(times: &mut [Duration]) -> Duration {
    times.sort_unstable();
    times[times.len() / 2]
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

    Maze::new(cells, edges).expect("1024x1024 square is a valid maze")
}
