use std::hint::black_box;
use std::time::Instant;

use labyrinth::{Cell, Maze, Model};

const SIDE: usize = 512;
const RUNS: usize = 3;

fn main() {
    let maze = open_square();
    let model = Model::new();
    let mut times = Vec::with_capacity(RUNS);
    let mut loops = Vec::with_capacity(RUNS);
    let mut token_passes = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        let start = Instant::now();
        let trace = black_box(model.trace(black_box(&maze)));
        times.push(start.elapsed().as_secs_f64() * 1_000.0);

        let path = trace.path.as_ref().expect("open square must be solvable");
        assert_eq!(path.len(), 2 * (SIDE - 1) + 1);
        assert_eq!(path.first(), Some(&0));
        assert_eq!(path.last(), Some(&(SIDE * SIDE - 1)));
        loops.push(trace.loops);
        token_passes.push(trace.token_passes);
    }

    times.sort_by(f64::total_cmp);
    loops.sort_unstable();
    token_passes.sort_unstable();
    println!("METRIC solve_ms={:.3}", times[RUNS / 2]);
    println!("METRIC loops={}", loops[RUNS / 2]);
    println!("METRIC token_passes={}", token_passes[RUNS / 2]);
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
