use std::hint::black_box;
use std::time::{Duration, Instant};

/// Each timed sample runs long enough to reduce timer noise on short solves.
const BATCH_TARGET: Duration = Duration::from_millis(10);
/// Caps batches when the calibration solve is below timer resolution.
const MAX_BATCH: u32 = 1_000_000;

pub fn time_batch(mut solve: impl FnMut() -> Option<Vec<usize>>) -> (Duration, Vec<usize>) {
    let calibration_start = Instant::now();
    black_box(solve()).expect("benchmark maze is solvable");
    let calibration_ns = calibration_start.elapsed().as_nanos().max(1);
    let repeats = (BATCH_TARGET.as_nanos() / calibration_ns).clamp(1, u128::from(MAX_BATCH)) as u32;

    let start = Instant::now();
    let mut path = Vec::new();
    for _ in 0..repeats {
        path = black_box(solve()).expect("benchmark maze is solvable");
    }
    (start.elapsed() / repeats, path)
}
