//! Browser bindings for maze generation and solving.

#[path = "../../server/src/board.rs"]
mod board;
#[path = "../../server/src/generate.rs"]
mod generate;

#[cfg(not(target_family = "wasm"))]
use std::time::Instant;

use board::{Board, Shape};
use generate::Algorithm;
use labyrinth::Model;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

const MIN_SIDE: usize = 5;
const MAX_SIDE: usize = 321;

#[derive(Serialize)]
struct Generated {
    cells: String,
    polygons: Option<Vec<Vec<f32>>>,
}

#[derive(Serialize)]
struct Solved {
    path: Option<Vec<usize>>,
    reached_at: Vec<Option<usize>>,
    loops: usize,
    tokens: usize,
    token_passes: usize,
    micros: u128,
    reached_cells: usize,
    open_cells: usize,
}

/// Generates a maze and its cell outlines as JSON.
#[wasm_bindgen]
pub fn generate_maze(
    shape: &str,
    algorithm: &str,
    width: usize,
    height: usize,
    seed: u64,
) -> Result<String, JsValue> {
    check_side(width).map_err(error)?;
    check_side(height).map_err(error)?;
    let shape = parse_shape(shape)?;
    let algorithm = parse_algorithm(algorithm)?;
    let board = Board::new(shape, width, height);
    let cells = generate::generate(&board, algorithm, seed);
    let polygons = (shape != Shape::Square).then(|| {
        board
            .polygons
            .iter()
            .map(|poly| {
                poly.iter()
                    .flat_map(|p| p.map(|v| ((v * 1000.0).round() / 1000.0) as f32))
                    .collect()
            })
            .collect()
    });
    json(&Generated {
        cells: String::from_utf8(cells).expect("generated cells are ASCII"),
        polygons,
    })
}

/// Solves the supplied maze and returns execution statistics as JSON.
#[wasm_bindgen]
pub fn solve_maze(
    shape: &str,
    width: usize,
    height: usize,
    cells: &str,
) -> Result<String, JsValue> {
    check_side(width).map_err(error)?;
    check_side(height).map_err(error)?;
    let board = Board::new(parse_shape(shape)?, width, height);
    let maze = board.maze(cells.as_bytes()).map_err(error)?;
    let tokens = maze.cells().len();
    let open_cells = maze
        .cells()
        .iter()
        .filter(|&&cell| cell != labyrinth::Cell::Wall)
        .count();
    #[cfg(target_family = "wasm")]
    let started = performance_now();
    #[cfg(not(target_family = "wasm"))]
    let started = Instant::now();
    let trace = Model::new().trace(&maze);
    #[cfg(target_family = "wasm")]
    let micros = ((performance_now() - started) * 1000.0) as u128;
    #[cfg(not(target_family = "wasm"))]
    let micros = started.elapsed().as_micros();
    let reached_cells = trace
        .reached_at
        .iter()
        .filter(|time| time.is_some())
        .count();
    json(&Solved {
        path: trace.path,
        reached_at: trace.reached_at,
        loops: trace.loops,
        tokens,
        token_passes: trace.token_passes,
        micros,
        reached_cells,
        open_cells,
    })
}

/// Returns the solver's hand-written parameter count.
#[wasm_bindgen]
pub fn parameter_count() -> usize {
    Model::new().parameter_count()
}

fn json(value: &impl Serialize) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(error)
}

fn error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn check_side(side: usize) -> Result<(), String> {
    if (MIN_SIDE..=MAX_SIDE).contains(&side) {
        Ok(())
    } else {
        Err(format!("side must be between {MIN_SIDE} and {MAX_SIDE}"))
    }
}

fn parse_shape(value: &str) -> Result<Shape, JsValue> {
    match value {
        "square" => Ok(Shape::Square),
        "hex" => Ok(Shape::Hex),
        "triangle" => Ok(Shape::Triangle),
        "penrose" => Ok(Shape::Penrose),
        "kite-dart" => Ok(Shape::KiteDart),
        _ => Err(error(format!("unknown shape: {value}"))),
    }
}

fn parse_algorithm(value: &str) -> Result<Algorithm, JsValue> {
    match value {
        "backtracker" => Ok(Algorithm::Backtracker),
        "prim" => Ok(Algorithm::Prim),
        "braid" => Ok(Algorithm::Braid),
        "caves" => Ok(Algorithm::Caves),
        _ => Err(error(format!("unknown algorithm: {value}"))),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_sides() {
        assert!(check_side(4).is_err());
    }
}
