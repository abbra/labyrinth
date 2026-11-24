//! A looped transformer whose weights are written by hand, not trained.
//!
//! Every maze cell is one token. One block (4-head attention + ReLU MLP) is applied
//! repeatedly. It floods outward from the goal: a cell becomes `REACHED` when a
//! neighbour is reached, and in that loop stores `PTR[d]`, the direction of the
//! neighbour that reached it first, so every pointer leads one BFS layer closer to the
//! goal. Once the start is reached its pointer chain is final and is the shortest path;
//! if the flood stops growing first, the goal is unreachable.
//!
//! Attention heads find the four grid neighbours through a relative-position bias, so
//! the same weights solve a maze of any size; the MLP snaps every update back to exact
//! 0/1 values, so softmax leakage never accumulates across loops.

use crate::maze::{Cell, Maze, Pos};

// Residual stream layout.
const ONE: usize = 0;
const OPEN: usize = 1;
const GOAL: usize = 2;
const REACHED: usize = 3;
const PTR: usize = 4;
const NBR_REACHED: usize = 8;
const D: usize = 12;

/// Attention sharpness. A token at squared distance `k ≥ 1` from a head's target gets
/// weight `e^(-BETA·k)`, so total leakage is ~`4e^-25`, below f32 resolution of the 0/1
/// features.
const BETA: f32 = 25.0;
/// Chebyshev radius of the relative-position bias table. Everything outside it is at
/// squared distance `≥ 5` from the target, whose weight `e^(-BETA·5)` is exactly 0 in f32.
const WINDOW: usize = 3;
/// Side of the relative-position bias table.
const SPAN: usize = 2 * WINDOW + 1;
/// Fewest tokens worth a thread of their own.
const MIN_CHUNK: usize = 512;
/// Slope of the snapping step; maps `x ≤ 0.45` to 0 and `x ≥ 0.55` to 1 exactly.
const STEEP: f32 = 10.0;

/// North, east, south, west as `(drow, dcol)`.
const DIRS: [(i64, i64); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];

type Vector = [f32; D];

fn dot(a: &Vector, b: &Vector) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// One attention head with a relative-position bias: the score of the token at offset
/// `(a - WINDOW, b - WINDOW)` is `bias[a][b]`.
#[derive(Debug, Clone)]
struct Head {
    bias: [[f32; SPAN]; SPAN],
    wv: Vector,
    wo: Vector,
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
        self.heads.len() * (SPAN * SPAN + 2 * D) + self.mlp.len() * 2 * D
    }

    /// Returns the shortest path from start to goal, both inclusive, or `None` when the
    /// goal is unreachable.
    ///
    /// Loops are bounded by the start's distance from the goal (or the flood depth when
    /// unreachable); each loop costs time proportional to the flood frontier.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn solve(&self, maze: &Maze) -> Option<Vec<Pos>> {
        self.trace(maze).path
    }

    /// Like [`Model::solve`], but also reports how the model got there.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn trace(&self, maze: &Maze) -> Trace {
        // A wall border gives every real cell four neighbours to attend to.
        let (h, w) = (maze.height() + 2, maze.width() + 2);
        let token = |p: Pos| (p.row + 1) * w + p.col + 1;
        let mut x = embed(maze, h, w);
        let mut reached_at = vec![None; maze.width() * maze.height()];
        // A token's output depends only on tokens within WINDOW of it, so one that saw no
        // change there last loop would recompute its own value: skip it.
        let mut dirty: Vec<usize> = (0..x.len()).collect();
        let mut marked = vec![false; x.len()];
        let (start, max_loops) = (token(maze.start()), x.len() + 1);
        let (mut loops, mut token_passes) = (0, 0);
        while x[start][REACHED] != 1.0 && !dirty.is_empty() {
            loops += 1;
            assert!(loops <= max_loops, "no fixed point after {max_loops} loops");
            token_passes += dirty.len();
            let next = self.step(&x, w, &dirty);
            let changed: Vec<usize> = dirty
                .iter()
                .zip(next)
                .filter_map(|(&i, t)| (std::mem::replace(&mut x[i], t) != t).then_some(i))
                .collect();
            dirty.clear();
            for i in changed {
                let (r, c) = (i / w, i % w);
                if (1..h - 1).contains(&r) && (1..w - 1).contains(&c) {
                    reached_at[(r - 1) * maze.width() + c - 1].get_or_insert(loops);
                }
                for row in r.saturating_sub(WINDOW)..(r + WINDOW + 1).min(h) {
                    let first = row * w + c.saturating_sub(WINDOW);
                    let span = &mut marked[first..row * w + (c + WINDOW + 1).min(w)];
                    for (j, seen) in (first..).zip(span) {
                        if !std::mem::replace(seen, true) {
                            dirty.push(j);
                        }
                    }
                }
            }
            dirty.iter().for_each(|&j| marked[j] = false);
        }
        Trace {
            path: read_path(maze, |p| &x[token(p)]),
            loops,
            token_passes,
            reached_at,
        }
    }

    /// One loop iteration over the `dirty` tokens of a padded grid `w` tokens wide,
    /// split across threads when there are enough of them.
    fn step(&self, x: &[Vector], w: usize, dirty: &[usize]) -> Vec<Vector> {
        let threads = std::thread::available_parallelism().map_or(1, usize::from);
        let chunk = dirty.len().div_ceil(threads).max(MIN_CHUNK);
        if dirty.len() <= chunk {
            return dirty.iter().map(|&i| self.block(i, x, w)).collect();
        }
        let mut next = vec![[0.0; D]; dirty.len()];
        std::thread::scope(|s| {
            for (out, ids) in next.chunks_mut(chunk).zip(dirty.chunks(chunk)) {
                s.spawn(move || {
                    for (t, &i) in out.iter_mut().zip(ids) {
                        *t = self.block(i, x, w);
                    }
                });
            }
        });
        next
    }

    /// Token `i` through the block: `t = x[i] + attn(x)[i]; t += mlp(t)`.
    fn block(&self, i: usize, x: &[Vector], w: usize) -> Vector {
        let (h, r, c) = (x.len() / w, i / w, i % w);
        // `(bias index, token)` for every in-grid offset of the bias table.
        let window = || {
            (0..SPAN)
                .flat_map(|a| (0..SPAN).map(move |b| (a, b)))
                .filter_map(move |(a, b)| {
                    let (row, col) = ((r + a).checked_sub(WINDOW)?, (c + b).checked_sub(WINDOW)?);
                    (row < h && col < w).then_some((a, b, row * w + col))
                })
        };
        let mut t = x[i];
        let mut attn = [0.0; D];
        for head in &self.heads {
            let max = window()
                .map(|(a, b, _)| head.bias[a][b])
                .fold(f32::NEG_INFINITY, f32::max);
            let (mut total, mut mix) = (0.0, 0.0);
            for (a, b, j) in window() {
                let s = head.bias[a][b] - max;
                // Below this e^x is exactly 0 in f32, so skipping it changes no sum.
                if s < -104.0 {
                    continue;
                }
                let e = s.exp();
                total += e;
                mix += e * dot(&head.wv, &x[j]);
            }
            for (o, wo) in attn.iter_mut().zip(&head.wo) {
                *o += mix / total * wo;
            }
        }
        t.iter_mut().zip(&attn).for_each(|(a, b)| *a += b);
        let mut out = [0.0; D];
        for unit in &self.mlp {
            let a = dot(&unit.input, &t).max(0.0);
            out.iter_mut()
                .zip(&unit.output)
                .for_each(|(o, w)| *o += a * w);
        }
        t.iter_mut().zip(&out).for_each(|(a, b)| *a += b);
        t
    }
}

/// What [`Model::trace`] observed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trace {
    /// Same as [`Model::solve`].
    pub path: Option<Vec<Pos>>,
    /// Loop iterations until the start was reached or the flood stopped growing.
    pub loops: usize,
    /// Token evaluations by the block, summed over all loops; tokens whose
    /// neighbourhood did not change are not re-evaluated.
    pub token_passes: usize,
    /// Row-major per cell: the loop in which the flood fill from the goal reached it.
    pub reached_at: Vec<Option<usize>>,
}

/// Follows `PTR` from a reached start to the goal.
fn read_path<'a>(maze: &Maze, token: impl Fn(Pos) -> &'a Vector) -> Option<Vec<Pos>> {
    let mut pos = maze.start();
    if token(pos)[REACHED] != 1.0 {
        return None;
    }
    let mut path = vec![pos];
    while pos != maze.goal() {
        let dir = (0..4)
            .find(|&d| token(pos)[PTR + d] == 1.0)
            .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
        let (dr, dc) = DIRS[dir];
        // Pointers only lead to reached, hence open, cells, which lie inside the maze.
        pos = Pos {
            row: pos.row.wrapping_add_signed(dr as isize),
            col: pos.col.wrapping_add_signed(dc as isize),
        };
        path.push(pos);
    }
    Some(path)
}

/// Token embedding of the padded `h × w` grid: just the cell type.
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
            t[ONE] = 1.0;
            t[OPEN] = f32::from(cell != Cell::Wall);
            t[GOAL] = f32::from(cell == Cell::Goal);
            x.push(t);
        }
    }
    x
}

/// Head `d` attends to the neighbour in direction `d`: the bias is
/// `-BETA·|offset - δ|²`, maximal (zero) exactly at that neighbour.
/// It copies the neighbour's `REACHED`.
fn head(d: usize) -> Head {
    let (dr, dc) = DIRS[d];
    let bias = std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let (er, ec) = (a as i64 - WINDOW as i64 - dr, b as i64 - WINDOW as i64 - dc);
            -BETA * (er * er + ec * ec) as f32
        })
    });
    let mut wv = [0.0; D];
    wv[REACHED] = 1.0;
    let mut wo = [0.0; D];
    wo[NBR_REACHED + d] = 1.0;
    Head { bias, wv, wo }
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
    // REACHED is overwritten, so it subtracts its old value; attention scratch is cleared.
    for i in [REACHED].into_iter().chain(NBR_REACHED..D) {
        let (mut input, mut output) = ([0.0; D], [0.0; D]);
        input[i] = 1.0;
        output[i] = -1.0;
        units.push(Unit { input, output });
    }
    units
}
