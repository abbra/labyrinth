//! A looped transformer whose weights are written by hand, not trained.
//!
//! Every maze cell is one token. One block (4-head attention + ReLU MLP) is applied
//! repeatedly until the residual stream stops changing:
//!
//! 1. Flood fill from the goal: a cell becomes `REACHED` when a neighbour is reached.
//!    In the step it is reached it stores `PTR[d]`, the direction of the neighbour that
//!    reached it first, so every pointer leads one BFS layer closer to the goal.
//! 2. Path walk from the start: the start becomes `ON_PATH` once reached; a cell becomes
//!    `ON_PATH` when a neighbour on the path points at it (`WALK`).
//!
//! Attention heads look up the four grid neighbours by position alone, so the same
//! weights solve any maze; the MLP snaps every update back to exact 0/1 values, so
//! softmax leakage never accumulates across loops.

use crate::maze::{Cell, Maze, Pos};

// Residual stream layout.
const ONE: usize = 0;
const ROW: usize = 1;
const COL: usize = 2;
/// `row² + col²`, lets a dot product express negative squared distance.
const SQ: usize = 3;
const OPEN: usize = 4;
const GOAL: usize = 5;
const START: usize = 6;
const REACHED: usize = 7;
const PTR: usize = 8;
const ON_PATH: usize = 12;
const WALK: usize = 13;
const NBR_REACHED: usize = 17;
const NBR_WALK: usize = 21;
const D: usize = 25;

/// Query/key width: `[row, col, row² + col², 1]`.
const HEAD: usize = 4;
/// Attention sharpness. A wrong token at squared distance `k ≥ 1` from the target gets
/// weight `e^(-BETA·k)`, so total leakage is ~`4e^-25` regardless of maze size and falls
/// below f32 resolution of the 0/1 features. Score magnitude grows as
/// `BETA·(row² + col²)`; f32 keeps the gap of `BETA` resolvable up to a few hundred
/// cells per side.
const BETA: f32 = 25.0;
/// Slope of the snapping step; maps `x ≤ 0.45` to 0 and `x ≥ 0.55` to 1 exactly.
const STEEP: f32 = 10.0;

/// North, east, south, west as `(drow, dcol)`.
const DIRS: [(i64, i64); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];

type Vector = [f32; D];

fn dot(a: &Vector, b: &Vector) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// One attention head; output dims are `D` rows so `wo[v]` is the write for value `v`.
#[derive(Debug, Clone)]
struct Head {
    wq: [Vector; HEAD],
    wk: [Vector; HEAD],
    wv: [Vector; 2],
    wo: [Vector; 2],
}

/// One ReLU unit: reads `input · x`, adds `relu(...) * output` to the residual.
#[derive(Debug, Clone)]
struct Unit {
    input: Vector,
    output: Vector,
}

/// The compiled maze-solving transformer.
///
/// The weights are fixed at construction and independent of the maze.
///
/// # Examples
///
/// ```
/// use labyrinth::{Maze, Model};
/// let maze: Maze = "S.#\n#.#\n#.G".parse()?;
/// let path = Model::new().solve(&maze).expect("maze is solvable");
/// assert_eq!(path.len(), 5);
/// # Ok::<(), labyrinth::ParseMazeError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Model {
    heads: [Head; 4],
    mlp: Vec<Unit>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Writes all weights.
    pub fn new() -> Self {
        Self {
            heads: std::array::from_fn(head),
            mlp: mlp(),
        }
    }

    /// Number of weights, biases included.
    pub fn parameter_count(&self) -> usize {
        self.heads.len() * (2 * HEAD + 4) * D + self.mlp.len() * 2 * D
    }

    /// Returns the shortest path from start to goal, both inclusive, or `None` when the
    /// goal is unreachable.
    ///
    /// Cost is `O(cells²)` per loop (dense attention) and `O(cells)` loops.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to converge, which would be a bug in the weights.
    pub fn solve(&self, maze: &Maze) -> Option<Vec<Pos>> {
        // A wall border gives every real cell four neighbours to attend to.
        let (h, w) = (maze.height() + 2, maze.width() + 2);
        let mut x = embed(maze, h, w);
        let max_loops = 3 * x.len() + 8;
        let converged = (0..max_loops).any(|_| {
            let before = x.clone();
            self.step(&mut x);
            x == before
        });
        assert!(converged, "no fixed point after {max_loops} loops");

        let token = |p: Pos| &x[(p.row + 1) * w + p.col + 1];
        let mut pos = maze.start();
        let mut path = Vec::new();
        while token(pos)[ON_PATH] == 1.0 {
            path.push(pos);
            if pos == maze.goal() {
                return Some(path);
            }
            let dir = (0..4).find(|&d| token(pos)[PTR + d] == 1.0)?;
            let (dr, dc) = DIRS[dir];
            pos = Pos {
                row: pos.row.checked_add_signed(dr as isize)?,
                col: pos.col.checked_add_signed(dc as isize)?,
            };
        }
        assert!(path.is_empty(), "path broke off after {path:?}");
        None
    }

    /// One loop iteration: `x += attn(x); x += mlp(x)`.
    fn step(&self, x: &mut [Vector]) {
        let mut delta = vec![[0.0; D]; x.len()];
        for head in &self.heads {
            let proj = |m: &[Vector; HEAD], t: &Vector| m.map(|row| dot(&row, t));
            let q: Vec<_> = x.iter().map(|t| proj(&head.wq, t)).collect();
            let k: Vec<_> = x.iter().map(|t| proj(&head.wk, t)).collect();
            let v: Vec<_> = x.iter().map(|t| head.wv.map(|row| dot(&row, t))).collect();
            let mut scores = vec![0.0; x.len()];
            for (qi, out) in q.iter().zip(&mut delta) {
                for (s, kj) in scores.iter_mut().zip(&k) {
                    *s = qi.iter().zip(kj).map(|(a, b)| a * b).sum();
                }
                let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let (mut total, mut mix) = (0.0, [0.0; 2]);
                for (s, vj) in scores.iter().zip(&v) {
                    let e = (s - max).exp();
                    total += e;
                    mix[0] += e * vj[0];
                    mix[1] += e * vj[1];
                }
                for (m, wo) in mix.iter().zip(&head.wo) {
                    for (o, w) in out.iter_mut().zip(wo) {
                        *o += m / total * w;
                    }
                }
            }
        }
        for (t, d) in x.iter_mut().zip(&delta) {
            t.iter_mut().zip(d).for_each(|(a, b)| *a += b);
        }
        for t in x.iter_mut() {
            let mut out = [0.0; D];
            for unit in &self.mlp {
                let a = dot(&unit.input, t).max(0.0);
                out.iter_mut()
                    .zip(&unit.output)
                    .for_each(|(o, w)| *o += a * w);
            }
            t.iter_mut().zip(&out).for_each(|(a, b)| *a += b);
        }
    }
}

/// Token embedding of the padded `h × w` grid: cell type plus position features.
fn embed(maze: &Maze, h: usize, w: usize) -> Vec<Vector> {
    let mut x = Vec::with_capacity(h * w);
    for row in 0..h {
        for col in 0..w {
            let cell = (row > 0 && col > 0)
                .then(|| {
                    maze.cell(Pos {
                        row: row - 1,
                        col: col - 1,
                    })
                })
                .flatten()
                .unwrap_or(Cell::Wall);
            let mut t = [0.0; D];
            let (r, c) = (row as f32, col as f32);
            t[ONE] = 1.0;
            t[ROW] = r;
            t[COL] = c;
            t[SQ] = r * r + c * c;
            t[OPEN] = f32::from(cell != Cell::Wall);
            t[GOAL] = f32::from(cell == Cell::Goal);
            t[START] = f32::from(cell == Cell::Start);
            x.push(t);
        }
    }
    x
}

/// Head `d` attends to the neighbour in direction `d`: the score is
/// `-BETA·|p + δ - p'|²`, maximal (zero) exactly at that neighbour.
/// It copies the neighbour's `REACHED` and whether it walks back toward us.
fn head(d: usize) -> Head {
    let (dr, dc) = (DIRS[d].0 as f32, DIRS[d].1 as f32);
    let mut wq = [[0.0; D]; HEAD];
    wq[0][ROW] = 2.0 * BETA;
    wq[0][ONE] = 2.0 * BETA * dr;
    wq[1][COL] = 2.0 * BETA;
    wq[1][ONE] = 2.0 * BETA * dc;
    wq[2][ONE] = -BETA;
    wq[3][SQ] = -BETA;
    wq[3][ROW] = -2.0 * BETA * dr;
    wq[3][COL] = -2.0 * BETA * dc;
    wq[3][ONE] = -BETA * (dr * dr + dc * dc);
    let mut wk = [[0.0; D]; HEAD];
    wk[0][ROW] = 1.0;
    wk[1][COL] = 1.0;
    wk[2][SQ] = 1.0;
    wk[3][ONE] = 1.0;
    let mut wv = [[0.0; D]; 2];
    wv[0][REACHED] = 1.0;
    wv[1][WALK + (d + 2) % 4] = 1.0;
    let mut wo = [[0.0; D]; 2];
    wo[0][NBR_REACHED + d] = 1.0;
    wo[1][NBR_WALK + d] = 1.0;
    Head { wq, wk, wv, wo }
}

/// Builds the MLP: each rule is a boolean `step(Σ terms + bias)` over 0/1 features.
fn mlp() -> Vec<Unit> {
    let mut units = Vec::new();
    // step(z) = relu(STEEP·(z-½)+½) - relu(STEEP·(z-½)-½), exact 0/1 away from z = ½.
    let mut step = |terms: &[(usize, f32)], bias: f32, target: usize| {
        for (offset, sign) in [(0.5, 1.0), (-0.5, -1.0)] {
            let mut input = [0.0; D];
            for &(i, wt) in terms {
                input[i] = STEEP * wt;
            }
            input[ONE] += STEEP * (bias - 0.5) + offset;
            let mut output = [0.0; D];
            output[target] = sign;
            units.push(Unit { input, output });
        }
    };
    // reached ← goal ∨ (open ∧ ∃ reached neighbour)
    let any_nbr: Vec<_> = (0..4).map(|d| (NBR_REACHED + d, 1.0)).collect();
    step(
        &[&any_nbr[..], &[(GOAL, 5.0), (OPEN, 5.0)]].concat(),
        -5.0,
        REACHED,
    );
    // ptr[d] += open ∧ ¬reached ∧ ¬goal ∧ nbr[d] ∧ ¬nbr[e] for e < d (first wins)
    for d in 0..4 {
        let mut terms = vec![
            (NBR_REACHED + d, 1.0),
            (REACHED, -1.0),
            (GOAL, -1.0),
            (OPEN, 1.0),
        ];
        terms.extend((0..d).map(|e| (NBR_REACHED + e, -1.0)));
        step(&terms, -1.0, PTR + d);
    }
    // on_path ← (start ∧ reached) ∨ ∃ neighbour walking to us
    let any_walk: Vec<_> = (0..4).map(|d| (NBR_WALK + d, 1.0)).collect();
    step(
        &[&any_walk[..], &[(START, 1.0), (REACHED, 1.0)]].concat(),
        -1.0,
        ON_PATH,
    );
    // walk[d] ← on_path ∧ ptr[d]
    for d in 0..4 {
        step(&[(ON_PATH, 1.0), (PTR + d, 1.0)], -1.0, WALK + d);
    }
    // Overwritten features subtract their old value; attention scratch is cleared.
    let replaced = [REACHED, ON_PATH].into_iter().chain(WALK..D);
    for i in replaced {
        let (mut input, mut output) = ([0.0; D], [0.0; D]);
        input[i] = 1.0;
        output[i] = -1.0;
        units.push(Unit { input, output });
    }
    units
}
