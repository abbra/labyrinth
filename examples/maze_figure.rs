//! Writes `paper/figures/mazes.svg`: benchmarked mazes with their `Model::solve` routes.
#[allow(dead_code)]
#[path = "../server/src/board.rs"]
mod board;
#[allow(dead_code)]
#[path = "../server/src/generate.rs"]
mod generate;

use std::fmt::Write;

use labyrinth::{Cell, Maze, Model};

/// Panel width in SVG units; every maze is scaled to it.
const PANEL: f64 = 256.0;
const GAP: f64 = 16.0;

fn main() {
    let panels = [backtracker(64), backtracker(256), open_square(64)];
    let mut svg = String::new();
    let width = panels.len() as f64 * (PANEL + GAP) - GAP;
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {PANEL}" width="{width}" height="{PANEL}" shape-rendering="crispEdges">"#
    )
    .unwrap();
    for (i, (side, maze)) in panels.iter().enumerate() {
        let path = Model::new()
            .solve(maze)
            .expect("benchmarked mazes are solvable");
        let scale = PANEL / *side as f64;
        let x = i as f64 * (PANEL + GAP);
        writeln!(svg, r#"<g transform="translate({x} 0) scale({scale})">"#).unwrap();
        // Hairline frame so the wall-less open square is visible on white paper.
        let frame = 1.0 / scale;
        writeln!(
            svg,
            r##"<rect width="{side}" height="{side}" fill="#ffffff" stroke="#999999" stroke-width="{frame}"/>"##
        )
        .unwrap();
        // One rect per horizontal run of walls keeps the 256² panel small.
        for row in 0..*side {
            let cells = &maze.cells()[row * side..(row + 1) * side];
            let mut col = 0;
            while col < *side {
                let run = cells[col..]
                    .iter()
                    .take_while(|&&c| c == Cell::Wall)
                    .count();
                if run > 0 {
                    writeln!(
                        svg,
                        r##"<rect x="{col}" y="{row}" width="{run}" height="1" fill="#222222"/>"##
                    )
                    .unwrap();
                }
                col += run.max(1);
            }
        }
        for &cell in &path {
            let colour = match maze.cells()[cell] {
                Cell::Start => "#1a9850",
                Cell::Goal => "#2166ac",
                _ => "#d73027",
            };
            let (row, col) = (cell / side, cell % side);
            writeln!(
                svg,
                r#"<rect x="{col}" y="{row}" width="1" height="1" fill="{colour}"/>"#
            )
            .unwrap();
        }
        writeln!(svg, "</g>").unwrap();
        println!("panel {i}: {side}x{side}, route {} cells", path.len());
    }
    svg.push_str("</svg>\n");
    std::fs::create_dir_all("paper/figures").unwrap();
    std::fs::write("paper/figures/mazes.svg", svg).unwrap();
}

/// Seed 1 of the benchmark's backtracker sweep.
fn backtracker(side: usize) -> (usize, Maze) {
    let board = board::Board {
        shape: board::Shape::Square,
        width: side,
        height: side,
        polygons: Vec::new(),
        neighbours: Vec::new(),
    };
    let cells = generate::generate(&board, generate::Algorithm::Backtracker, 1)
        .into_iter()
        .map(|cell| match cell {
            b'#' => Cell::Wall,
            b'.' => Cell::Open,
            b'S' => Cell::Start,
            b'G' => Cell::Goal,
            _ => panic!("unexpected generated cell {}", cell as char),
        })
        .collect();
    (side, Maze::new(cells, edges(side)).unwrap())
}

fn open_square(side: usize) -> (usize, Maze) {
    let mut cells = vec![Cell::Open; side * side];
    cells[0] = Cell::Start;
    cells[side * side - 1] = Cell::Goal;
    (side, Maze::new(cells, edges(side)).unwrap())
}

fn edges(side: usize) -> Vec<(usize, usize)> {
    (0..side * side)
        .flat_map(|cell| {
            let right = (cell % side + 1 < side).then_some((cell, cell + 1));
            let down = (cell / side + 1 < side).then_some((cell, cell + side));
            right.into_iter().chain(down)
        })
        .collect()
}
