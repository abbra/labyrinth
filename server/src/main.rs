//! Web front end for the hand-weighted maze-solving transformer.
//!
//! `GET /` serves the single-page UI, `POST /api/generate` builds a maze and
//! `POST /api/solve` runs the model and returns the path plus execution statistics.

mod board;
mod generate;

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use board::{Board, Shape};
use generate::Algorithm;
use labyrinth::Model;
use serde::{Deserialize, Serialize};

/// Smallest side the carving generators can fit separate start and goal rooms into.
const MIN_SIDE: usize = 5;
/// Largest side accepted; a 321×321 maze of any shape solves in about 0.3 s on 24 cores.
const MAX_SIDE: usize = 321;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::var("LABYRINTH_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("index.html")) }))
        .route("/api/model", get(model_info))
        .route("/api/generate", post(generate))
        .route("/api/solve", post(solve))
        .with_state(Arc::new(Model::new()));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await
}

/// A request the client got wrong; rendered as `400` with the message as body.
struct BadRequest(String);

impl IntoResponse for BadRequest {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

fn check_side(name: &str, value: usize) -> Result<(), BadRequest> {
    if (MIN_SIDE..=MAX_SIDE).contains(&value) {
        Ok(())
    } else {
        Err(BadRequest(format!(
            "{name} must be between {MIN_SIDE} and {MAX_SIDE}"
        )))
    }
}

#[derive(Serialize)]
struct ModelInfo {
    parameters: usize,
    max_side: usize,
}

async fn model_info(State(model): State<Arc<Model>>) -> Json<ModelInfo> {
    Json(ModelInfo {
        parameters: model.parameter_count(),
        max_side: MAX_SIDE,
    })
}

#[derive(Deserialize)]
struct GenerateRequest {
    shape: Shape,
    algorithm: Algorithm,
    width: usize,
    height: usize,
    seed: u64,
}

#[derive(Serialize)]
struct GenerateResponse {
    /// One of `#.SG` per cell; square boards are row-major.
    cells: String,
    /// Flat `[x0, y0, x1, y1, …]` outline per cell, omitted for square boards, whose
    /// geometry the client derives from the width.
    polygons: Option<Vec<Vec<f32>>>,
}

async fn generate(Json(req): Json<GenerateRequest>) -> Result<Json<GenerateResponse>, BadRequest> {
    check_side("width", req.width)?;
    check_side("height", req.height)?;
    let board = Board::new(req.shape, req.width, req.height);
    let cells = generate::generate(&board, req.algorithm, req.seed);
    let polygons = (board.shape != Shape::Square).then(|| {
        // Millipixel precision keeps the JSON short.
        let round = |v: f64| ((v * 1000.0).round() / 1000.0) as f32;
        board
            .polygons
            .iter()
            .map(|poly| poly.iter().flat_map(|p| p.map(round)).collect())
            .collect()
    });
    Ok(Json(GenerateResponse {
        cells: String::from_utf8(cells).expect("cells are ASCII"),
        polygons,
    }))
}

#[derive(Deserialize)]
struct SolveRequest {
    shape: Shape,
    width: usize,
    height: usize,
    cells: String,
}

#[derive(Serialize)]
struct SolveResponse {
    /// Cell ids from start to goal, or `None` when the goal is unreachable.
    path: Option<Vec<usize>>,
    /// Loop index at which the goal flood reached each cell.
    reached_at: Vec<Option<usize>>,
    /// Transformer block applications until the start was reached or the flood stopped.
    loops: usize,
    /// Tokens in the sequence: one per cell.
    tokens: usize,
    /// Token evaluations actually run, summed over loops.
    token_passes: usize,
    open_cells: usize,
    reached_cells: usize,
    micros: u128,
}

async fn solve(
    State(model): State<Arc<Model>>,
    Json(req): Json<SolveRequest>,
) -> Result<Json<SolveResponse>, BadRequest> {
    check_side("width", req.width)?;
    check_side("height", req.height)?;
    let board = Board::new(req.shape, req.width, req.height);
    let maze = board
        .maze(req.cells.as_bytes())
        .map_err(|e| BadRequest(format!("invalid maze: {e}")))?;
    let tokens = board.len();
    let open_cells = req.cells.bytes().filter(|&b| b != b'#').count();
    // The model loop is CPU-bound and multi-threaded; keep it off the async workers.
    let (trace, micros) = tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let trace = model.trace(&maze);
        (trace, started.elapsed().as_micros())
    })
    .await
    .expect("solver panicked");
    Ok(Json(SolveResponse {
        path: trace.path,
        reached_cells: trace.reached_at.iter().flatten().count(),
        reached_at: trace.reached_at,
        token_passes: trace.token_passes,
        loops: trace.loops,
        tokens,
        open_cells,
        micros,
    }))
}
