use std::hint::black_box;
use std::time::{Duration, Instant};

/// Each timed sample runs long enough to reduce timer noise on short solves.
const BATCH_TARGET: Duration = Duration::from_millis(10);
/// Caps batches when the calibration solve is below timer resolution.
const MAX_BATCH: u32 = 1_000_000;

pub fn time_batch(mut solve: impl FnMut() -> Option<Vec<usize>>) -> (Duration, Vec<usize>) {
    let start = Instant::now();
    let mut path = black_box(solve()).expect("benchmark maze is solvable");
    let first = start.elapsed();
    let repeats =
        (BATCH_TARGET.as_nanos() / first.as_nanos().max(1)).clamp(1, u128::from(MAX_BATCH)) as u32;
    if repeats == 1 {
        return (first, path);
    }
    for _ in 1..repeats {
        path = black_box(solve()).expect("benchmark maze is solvable");
    }
    (start.elapsed() / repeats, path)
}
