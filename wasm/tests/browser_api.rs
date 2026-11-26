use labyrinth_wasm::{generate_maze, solve_maze};
use serde_json::Value;

#[test]
fn generates_and_solves_without_server_state() {
    let generated: Value = serde_json::from_str(
        &generate_maze("square", "backtracker", 5, 5, 42).expect("valid request"),
    )
    .expect("generation JSON");
    let cells = generated["cells"].as_str().expect("cell string");
    let model: Value =
        serde_json::from_str(&solve_maze("square", 5, 5, cells, "model").expect("valid maze"))
            .expect("solve JSON");
    assert!(model["path"].is_array());
    assert_eq!(model["tokens"], 25);
    assert_eq!(
        model["open_cells"],
        cells.bytes().filter(|&cell| cell != b'#').count()
    );
    // The classical search returns exactly the model's route.
    let bfs: Value =
        serde_json::from_str(&solve_maze("square", 5, 5, cells, "bfs").expect("valid maze"))
            .expect("bfs JSON");
    assert_eq!(bfs["path"], model["path"]);
    // Unknown solver names and a_star's square-only heuristic are rejected
    // (asserted in solvers' unit tests; building the JsValue error here would
    // panic on non-wasm32 targets).
    let hex = serde_json::from_str::<Value>(
        &generate_maze("hex", "backtracker", 7, 7, 42).expect("valid request"),
    )
    .expect("generation JSON");
    let hex_cells = hex["cells"].as_str().expect("cell string");
    let transformer: Value = serde_json::from_str(
        &solve_maze("hex", 7, 7, hex_cells, "transformer").expect("valid maze"),
    )
    .expect("transformer JSON");
    assert!(transformer["path"].is_array());
}
