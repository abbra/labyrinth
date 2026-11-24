//! Web front end for the hand-weighted maze-solving transformer.
//!
//! `GET /` serves the single-page UI, `POST /api/generate` builds a maze and
//! `POST /api/solve` runs the model and returns the path plus execution statistics.

mod generate;

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use generate::Algorithm;
use labyrinth::{Maze, Model};
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
    algorithm: Algorithm,
    width: usize,
    height: usize,
    seed: u64,
}

#[derive(Serialize)]
struct GenerateResponse {
    rows: Vec<String>,
}

async fn generate(Json(req): Json<GenerateRequest>) -> Result<Json<GenerateResponse>, BadRequest> {
    check_side("width", req.width)?;
    check_side("height", req.height)?;
    let rows = generate::generate(req.algorithm, req.height, req.width, req.seed);
    Ok(Json(GenerateResponse { rows }))
}

#[derive(Deserialize)]
struct SolveRequest {
    rows: Vec<String>,
}

#[derive(Serialize)]
struct SolveResponse {
    /// `[row, col]` cells from start to goal, or `None` when the goal is unreachable.
    path: Option<Vec<[usize; 2]>>,
    /// Row-major loop index at which the goal flood reached each cell.
    reached_at: Vec<Option<usize>>,
    /// Transformer block applications until the start was reached or the flood stopped.
    loops: usize,
    /// Tokens in the padded grid: every cell plus the wall border.
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
    let maze: Maze = req
        .rows
        .join("\n")
        .parse()
        .map_err(|e| BadRequest(format!("invalid maze: {e}")))?;
    check_side("width", maze.width())?;
    check_side("height", maze.height())?;
    let tokens = (maze.width() + 2) * (maze.height() + 2);
    // The model loop is CPU-bound and multi-threaded; keep it off the async workers.
    let (trace, micros) = tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let trace = model.trace(&maze);
        (trace, started.elapsed().as_micros())
    })
    .await
    .expect("solver panicked");
    let open_cells = req
        .rows
        .iter()
        .flat_map(|r| r.bytes())
        .filter(|&b| b != b'#')
        .count();
    Ok(Json(SolveResponse {
        path: trace
            .path
            .map(|p| p.iter().map(|p| [p.row, p.col]).collect()),
        reached_cells: trace.reached_at.iter().flatten().count(),
        reached_at: trace.reached_at,
        token_passes: trace.token_passes,
        loops: trace.loops,
        tokens,
        open_cells,
        micros,
    }))
}
