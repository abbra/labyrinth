use labyrinth_wasm::{generate_maze, solve_maze};
use serde_json::Value;

#[test]
fn generates_and_solves_without_server_state() {
    let generated: Value = serde_json::from_str(
        &generate_maze("square", "backtracker", 5, 5, 42).expect("valid request"),
    )
    .expect("generation JSON");
    let cells = generated["cells"].as_str().expect("cell string");
    let solved: Value =
        serde_json::from_str(&solve_maze("square", 5, 5, cells).expect("valid maze"))
            .expect("solve JSON");
    assert!(solved["path"].is_array());
    assert_eq!(solved["tokens"], 25);
    assert_eq!(
        solved["open_cells"],
        cells.bytes().filter(|&cell| cell != b'#').count()
    );
}
