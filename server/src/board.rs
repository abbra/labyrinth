//! Tilings mazes are built on. A board is a list of cell polygons; two cells are
//! neighbours when they share a polygon edge, so every tiling goes through one code path.

use std::collections::HashMap;
use std::f64::consts::PI;

use labyrinth::{Cell, Maze};
use serde::Deserialize;

/// Cell shape of the tiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    Square,
    /// Pointy-top hexagons, odd rows shifted right.
    Hex,
    /// Alternating up and down triangles.
    Triangle,
    /// Aperiodic Penrose P3 rhombs (thick and thin), clipped to the board rectangle.
    Penrose,
}

pub type Point = [f64; 2];

pub struct Board {
    pub shape: Shape,
    pub width: usize,
    pub height: usize,
    /// Cell outlines; for square, hex and triangle boards cell ids are row-major.
    pub polygons: Vec<Vec<Point>>,
    pub neighbours: Vec<Vec<usize>>,
}

impl Board {
    /// `width × height` cells for the regular tilings; for Penrose a `width × height`
    /// rectangle measured in rhomb edges.
    pub fn new(shape: Shape, width: usize, height: usize) -> Self {
        let polygons = match shape {
            Shape::Square => grid(width, height, |r, c| {
                let (x, y) = (c as f64, r as f64);
                vec![[x, y], [x + 1.0, y], [x + 1.0, y + 1.0], [x, y + 1.0]]
            }),
            Shape::Hex => grid(width, height, |r, c| {
                let cx = 3f64.sqrt() * (c as f64 + 0.5 * (r % 2) as f64 + 0.5);
                let cy = 1.5 * r as f64 + 1.0;
                (0..6)
                    .map(|k| {
                        let a = PI / 6.0 + PI / 3.0 * k as f64;
                        [cx + a.cos(), cy + a.sin()]
                    })
                    .collect()
            }),
            Shape::Triangle => grid(width, height, |r, c| {
                let (x, h) = (0.5 * c as f64, 3f64.sqrt() / 2.0);
                let (top, bottom) = (h * r as f64, h * (r + 1) as f64);
                if (r + c) % 2 == 0 {
                    vec![[x + 0.5, top], [x + 1.0, bottom], [x, bottom]]
                } else {
                    vec![[x, top], [x + 1.0, top], [x + 0.5, bottom]]
                }
            }),
            Shape::Penrose => penrose(width as f64, height as f64),
        };
        let neighbours = neighbours(&polygons);
        Self {
            shape,
            width,
            height,
            polygons,
            neighbours,
        }
    }

    pub fn len(&self) -> usize {
        self.polygons.len()
    }

    /// Vertex average, good enough to order cells by position.
    pub fn centre(&self, cell: usize) -> Point {
        let poly = &self.polygons[cell];
        let n = poly.len() as f64;
        poly.iter()
            .fold([0.0, 0.0], |[x, y], p| [x + p[0] / n, y + p[1] / n])
    }

    /// Builds the maze for `cells`, one `#`, `.`, `S` or `G` per cell.
    pub fn maze(&self, cells: &[u8]) -> Result<Maze, String> {
        if cells.len() != self.len() {
            return Err(format!(
                "board has {} cells, got {}",
                self.len(),
                cells.len()
            ));
        }
        let cells = cells
            .iter()
            .map(|&b| match b {
                b'#' => Ok(Cell::Wall),
                b'.' => Ok(Cell::Open),
                b'S' => Ok(Cell::Start),
                b'G' => Ok(Cell::Goal),
                _ => Err(format!("unexpected cell {:?}", b as char)),
            })
            .collect::<Result<_, _>>()?;
        let edges = self
            .neighbours
            .iter()
            .enumerate()
            .flat_map(|(a, ns)| ns.iter().filter(move |&&b| a < b).map(move |&b| (a, b)));
        Maze::new(cells, edges).map_err(|e| e.to_string())
    }
}

fn grid(width: usize, height: usize, cell: impl Fn(usize, usize) -> Vec<Point>) -> Vec<Vec<Point>> {
    (0..height)
        .flat_map(|r| (0..width).map(move |c| (r, c)))
        .map(|(r, c)| cell(r, c))
        .collect()
}

/// Neighbour lists from shared polygon edges. Vertices are matched on a 1e-4 grid, far
/// coarser than the float error of the tilings above and far finer than their edges.
fn neighbours(polygons: &[Vec<Point>]) -> Vec<Vec<usize>> {
    let key = |[x, y]: Point| ((x * 1e4).round() as i64, (y * 1e4).round() as i64);
    let mut owner = HashMap::new();
    let mut neighbours = vec![Vec::new(); polygons.len()];
    for (i, poly) in polygons.iter().enumerate() {
        for (k, &p) in poly.iter().enumerate() {
            let (a, b) = (key(p), key(poly[(k + 1) % poly.len()]));
            if let Some(j) = owner.insert((a.min(b), a.max(b)), i) {
                neighbours[i].push(j);
                neighbours[j].push(i);
            }
        }
    }
    neighbours
}

/// Robinson-triangle deflation (Preshing's construction): a wheel of ten half-rhombs is
/// subdivided until edges are 1 long; pairs sharing their `B–C` side form the rhombs.
fn penrose(width: f64, height: f64) -> Vec<Vec<Point>> {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let lerp = |p: Point, q: Point, t: f64| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
    let (hw, hh) = (width / 2.0, height / 2.0);
    // Wheel radius φⁿ, so n deflations leave unit edges, and large enough to cover the board.
    let mut radius = 1.0;
    while radius < hw.hypot(hh) + 2.0 {
        radius *= phi;
    }
    // (thick, A, B, C): A is the apex, B–C the side shared with the other half.
    let mut tris: Vec<(bool, Point, Point, Point)> = (0..10)
        .map(|i| {
            let at = |k: f64| {
                [
                    radius * (k * PI / 10.0).cos(),
                    radius * (k * PI / 10.0).sin(),
                ]
            };
            let (b, c) = (at(2.0 * i as f64 - 1.0), at(2.0 * i as f64 + 1.0));
            if i % 2 == 0 {
                (false, [0.0, 0.0], c, b)
            } else {
                (false, [0.0, 0.0], b, c)
            }
        })
        .collect();
    // Children lie inside their parent, so triangles clear of the board can go early.
    let near = |ps: [Point; 3]| {
        let out = |f: &dyn Fn(Point) -> bool| ps.iter().all(|&p| f(p));
        !(out(&|p| p[0] < -hw - 1.0)
            || out(&|p| p[0] > hw + 1.0)
            || out(&|p| p[1] < -hh - 1.0)
            || out(&|p| p[1] > hh + 1.0))
    };
    while radius > 1.0 + 1e-9 {
        let mut next = Vec::with_capacity(tris.len() * 3);
        for (thick, a, b, c) in tris {
            if !thick {
                let p = lerp(a, b, 1.0 / phi);
                next.extend([(false, c, p, b), (true, p, c, a)]);
            } else {
                let q = lerp(b, a, 1.0 / phi);
                let r = lerp(b, c, 1.0 / phi);
                next.extend([(true, r, c, a), (true, q, r, b), (false, r, q, a)]);
            }
        }
        next.retain(|&(_, a, b, c)| near([a, b, c]));
        tris = next;
        radius /= phi;
    }
    let key = |[x, y]: Point| ((x * 1e4).round() as i64, (y * 1e4).round() as i64);
    let mut halves = HashMap::new();
    let mut rhombs = Vec::new();
    for (_, a, b, c) in tris {
        let side = (key(b).min(key(c)), key(b).max(key(c)));
        if let Some(other) = halves.insert(side, a) {
            let centre = lerp(b, c, 0.5);
            if centre[0].abs() <= hw && centre[1].abs() <= hh {
                rhombs.push([a, b, other, c].map(|p| [p[0] + hw, p[1] + hh]).to_vec());
            }
        }
    }
    rhombs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilings_have_expected_degrees() {
        for (shape, max, inner) in [
            (Shape::Square, 4, 4),
            (Shape::Hex, 6, 6),
            (Shape::Triangle, 3, 3),
            (Shape::Penrose, 4, 4),
        ] {
            let board = Board::new(shape, 12, 9);
            let degrees: Vec<_> = board.neighbours.iter().map(Vec::len).collect();
            assert!(degrees.iter().all(|&d| d <= max), "{shape:?}: {degrees:?}");
            // Interior cells are fully surrounded, so shared edges were all matched.
            assert!(degrees.contains(&inner), "{shape:?}: {degrees:?}");
            assert!(board.len() > 40, "{shape:?}: {} cells", board.len());
        }
    }

    #[test]
    fn penrose_cells_are_unit_rhombs() {
        let board = Board::new(Shape::Penrose, 15, 10);
        for poly in &board.polygons {
            for k in 0..4 {
                let (p, q) = (poly[k], poly[(k + 1) % 4]);
                assert!(((p[0] - q[0]).hypot(p[1] - q[1]) - 1.0).abs() < 1e-6);
            }
        }
    }
}
