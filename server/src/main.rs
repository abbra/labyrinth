//! Web front end for the fixed-rule maze solver.
//!
//! `GET /` serves the single-page UI, `POST /api/generate` builds a maze and
//! `POST /api/solve` runs a selected solver and returns the path plus statistics.

mod alloc;
mod board;
mod generate;
mod solvers;

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Json, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use board::{Board, Shape};
use generate::Algorithm;
use labyrinth::Model;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

/// Smallest side the carving generators can fit separate start and goal rooms into.
const MIN_SIDE: usize = 5;
/// Largest side accepted; bounds board size and per-request resource use.
const MAX_SIDE: usize = 321;

/// Bounds concurrent model runs to limit CPU oversubscription.
const MAX_PARALLEL_SOLVES: usize = 2;

#[derive(Clone)]
struct AppState {
    model: Arc<Model>,
    solve_slots: Arc<Semaphore>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::var("LABYRINTH_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("index.html")) }))
        .route("/wasm/pkg/{file}", get(serve_pkg))
        .route("/api/model", get(model_info))
        .route("/api/generate", post(generate))
        .route("/api/solve", post(solve))
        .with_state(AppState {
            model: Arc::new(Model::new()),
            solve_slots: Arc::new(Semaphore::new(MAX_PARALLEL_SOLVES)),
        });
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await
}

/// Allowlisted wasm-bindgen bundle files for the `?backend=wasm` UI, read from
/// the workspace so the built pkg stays the single source of truth.
async fn serve_pkg(Path(file): Path<String>) -> Result<Response, StatusCode> {
    let mime = match file.as_str() {
        "labyrinth_wasm.js" => "application/javascript",
        "labyrinth_wasm_bg.wasm" => "application/wasm",
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../wasm/pkg/",);

    let body = std::fs::read(format!("{path}{file}")).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE, mime)], body).into_response())
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

async fn model_info(State(state): State<AppState>) -> Json<ModelInfo> {
    Json(ModelInfo {
        parameters: state.model.parameter_count(),
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
    tokio::task::spawn_blocking(move || {
        let board = Board::new(req.shape, req.width, req.height);
        let cells = generate::generate(&board, req.algorithm, req.seed);
        let polygons = (board.shape != Shape::Square).then(|| {
            let round = |v: f64| ((v * 1000.0).round() / 1000.0) as f32;
            board
                .polygons
                .iter()
                .map(|poly| poly.iter().flat_map(|p| p.map(round)).collect())
                .collect()
        });
        GenerateResponse {
            cells: String::from_utf8(cells).expect("cells are ASCII"),
            polygons,
        }
    })
    .await
    .map(Json)
    .map_err(|_| BadRequest("maze generation failed".into()))
}

#[derive(Deserialize)]
struct SolveRequest {
    shape: Shape,
    width: usize,
    height: usize,
    cells: String,
    /// Solver wire name; defaults to the model for older clients.
    #[serde(default = "default_solver")]
    solver: String,
}

fn default_solver() -> String {
    "model".into()
}

#[derive(Serialize)]
struct SolveResponse {
    /// Cell ids from start through every goal, or `None` when any goal is unreachable.
    path: Option<Vec<usize>>,
    /// Loop, wave or distance index at which the search reached each cell.
    reached_at: Vec<Option<usize>>,
    /// Model loop iterations, or the largest search wave or distance.
    loops: usize,
    /// Tokens in the sequence: one per cell.
    tokens: usize,
    /// Token evaluations actually run, summed over loops.
    token_passes: usize,
    open_cells: usize,
    reached_cells: usize,
    micros: u128,
    /// Heap use of the solve, from the counting allocator.
    #[serde(with = "UsageDef")]
    heap: alloc::Usage,
}

/// Serde mirror of `alloc::Usage`, which stays serde-free so the library bench can share it.
#[derive(Serialize)]
#[serde(remote = "alloc::Usage")]
struct UsageDef {
    allocs: u64,
    bytes: u64,
    peak: u64,
}

async fn solve(
    State(state): State<AppState>,
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
    let method = solvers::parse_method(&req.solver)
        .ok_or_else(|| BadRequest(solvers::unknown_method(&req.solver)))?;
    let square_width = (req.shape == Shape::Square).then_some(req.width);
    let permit = state
        .solve_slots
        .clone()
        .acquire_owned()
        .await
        .expect("solve semaphore remains open");
    let ((outcome, micros), heap) = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        alloc::measure(|| {
            let started = Instant::now();
            (
                solvers::solve(&maze, square_width, method),
                started.elapsed().as_micros(),
            )
        })
    })
    .await
    .expect("solver panicked");
    let outcome = outcome.map_err(BadRequest)?;
    Ok(Json(SolveResponse {
        path: outcome.path,
        reached_cells: outcome.reached_cells,
        reached_at: outcome.reached_at,
        token_passes: outcome.token_passes,
        loops: outcome.loops,
        tokens,
        open_cells,
        micros,
        heap,
    }))
}
