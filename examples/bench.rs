//! Reads a maze from stdin, solves it once and prints time and heap use.

use std::io::{self, Read};
use std::time::Instant;

use labyrinth::{Maze, Model};

// One counting allocator for the server and this bench; it uses only `std`.
#[path = "../server/src/alloc.rs"]
mod alloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    let maze: Maze = text.parse()?;
    let model = Model::new();
    let tokens = maze.cells().len();
    let start = Instant::now();
    let (trace, heap) = alloc::measure(|| model.trace(&maze));
    let secs = start.elapsed().as_secs_f64();
    let mib = |b: u64| b as f64 / f64::from(1 << 20);
    println!(
        "tokens={tokens} loops={} path={} secs={secs:.3} allocs={} allocated={:.1}MiB peak={:.1}MiB peak/token={}B",
        trace.loops,
        trace.path.map_or(0, |p| p.len()),
        heap.allocs,
        mib(heap.bytes),
        mib(heap.peak),
        heap.peak / tokens as u64,
    );
    Ok(())
}
