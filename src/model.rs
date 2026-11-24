//! A looped transformer whose weights are generated from fixed BFS rules.
//!
//! Every maze cell is one token. One block (attention with one head per neighbour slot +
//! ReLU MLP) is applied repeatedly. It floods outward from all goals: a cell becomes
//! `REACHED` when a neighbour is reached, and in that loop stores `PTR[k]`, the slot of
//! the neighbour that reached it first, so every pointer leads one BFS layer closer to
//! a goal. Once the start is reached its pointer chain reaches the nearest goal. The
//! solver then joins shortest paths to the remaining goals; if any is unreachable, no
//! complete route exists.
//!
//! The maze is a graph, so attention uses a structural bias in the style of graph
//! transformers: the score of key `j` for query `i` is a weight looked up by `j`'s slot in
//! `i`'s neighbour list, with an extra attention-sink token for missing slots and `-∞`
//! for everything else. The same weights therefore solve a maze of any size and any
//! tiling with at most [`MAX_DEGREE`] neighbours per cell; the MLP snaps every update
//! back to exact 0/1 values, so softmax leakage never accumulates across loops.

use std::any::Any;
use std::collections::VecDeque;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::sync::{Barrier, Mutex, PoisonError, RwLock};

use crate::maze::{Cell, MAX_DEGREE as K, Maze};

// Residual stream layout.
const ONE: usize = 0;
const OPEN: usize = 1;
const GOAL: usize = 2;
const REACHED: usize = 3;
const PTR: usize = 4;
const NBR_REACHED: usize = PTR + K;
const D: usize = NBR_REACHED + K;

/// Attention sharpness: each bias rank is `BETA` below the previous, so whichever key ranks
/// top leaks at most `K·e^-25` onto the rest, below f32 resolution of the 0/1 features.
const BETA: f32 = 25.0;
/// Bias slot of the attention sink, an all-zero token every cell can see.
const SINK: usize = K;
/// Dirty tokens a thread claims at a time.
const CLAIM: usize = 8;
/// Fewest dirty tokens worth waking the pool for; measured, not derived.
const PARALLEL_MIN: usize = 4 * CLAIM;
/// Slope of the snapping step; maps `x ≤ 0.45` to 0 and `x ≥ 0.55` to 1 exactly.
const STEEP: f32 = 10.0;

type Vector = [f32; D];

fn dot(a: &Vector, b: &Vector) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// One attention head with a structural bias: the score of a cell's `s`-th neighbour is
/// `bias[s]`, of the sink `bias[SINK]`; other tokens are masked out.
#[derive(Debug, Clone)]
struct Head {
    bias: [f32; K + 1],
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
/// assert_eq!(path, [0, 1, 4, 7, 8]);
/// # Ok::<(), labyrinth::MazeError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Model {
    heads: [Head; K],
    mlp: Vec<Unit>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Generates all weights from the fixed solver rules.
    pub fn new() -> Self {
        Self {
            heads: std::array::from_fn(head),
            mlp: mlp(),
        }
    }

    /// Number of weights, biases included.
    pub fn parameter_count(&self) -> usize {
        self.heads.len() * (K + 1 + 2 * D) + self.mlp.len() * 2 * D
    }

    /// Returns a route from the start through every goal, or `None` when any goal is
    /// unreachable from the route.
    ///
    /// For multiple goals, it visits the nearest unvisited goal next, using a shortest
    /// path for each leg.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        self.trace(maze).path
    }

    /// Like [`Model::solve`], but also reports how the model got there.
    ///
    /// # Panics
    ///
    /// Panics if the model fails to terminate, which would be a bug in the weights.
    pub fn trace(&self, maze: &Maze) -> Trace {
        let x = embed(maze);
        let threads = std::thread::available_parallelism().map_or(1, usize::from);
        let pool = Pool {
            // A token's output depends only on itself and its neighbours, so one that saw
            // no change there last loop would recompute its own value: skip it.
            dirty: RwLock::new((0..x.len()).collect()),
            x: RwLock::new(x),
            cursor: AtomicUsize::new(0),
            out: (0..threads).map(|_| Mutex::default()).collect(),
            start: Barrier::new(threads),
            end: Barrier::new(threads),
            done: AtomicBool::new(false),
            panic: Mutex::new(None),
        };
        let mut reached_at = vec![None; maze.cells().len()];
        let (loops, token_passes) = std::thread::scope(|s| {
            for id in 1..threads {
                let pool = &pool;
                s.spawn(move || {
                    while pool.wait_for_loop() {
                        pool.work(self, maze, id);
                        pool.end.wait();
                    }
                });
            }
            // Releases the workers on every exit, unwinding included, so scope can join.
            let _shutdown = Shutdown(&pool);
            let n = maze.cells().len();
            let mut marked = vec![false; n];
            let mut changed = Vec::new();
            let (start, max_loops) = (maze.start(), n + 1);
            let (mut loops, mut token_passes) = (0, 0);
            loop {
                let (mut x, mut dirty) = (pool.x.write().unwrap(), pool.dirty.write().unwrap());
                if x[start][REACHED] == 1.0 || dirty.is_empty() {
                    break (loops, token_passes);
                }
                loops += 1;
                assert!(loops <= max_loops, "no fixed point after {max_loops} loops");
                token_passes += dirty.len();
                let parallel = threads > 1 && dirty.len() >= PARALLEL_MIN;
                drop((x, dirty));
                pool.cursor.store(0, Relaxed);
                if parallel {
                    pool.start.wait();
                    pool.work(self, maze, 0);
                    pool.end.wait();
                } else {
                    pool.work(self, maze, 0);
                }
                if let Some(payload) = pool.panic.lock().unwrap().take() {
                    panic::resume_unwind(payload);
                }
                (x, dirty) = (pool.x.write().unwrap(), pool.dirty.write().unwrap());
                for out in &pool.out {
                    for (i, t) in out.lock().unwrap().drain(..) {
                        if std::mem::replace(&mut x[i], t) != t {
                            changed.push(i);
                        }
                    }
                }
                dirty.clear();
                for i in changed.drain(..) {
                    reached_at[i].get_or_insert(loops);
                    for &j in std::iter::once(&i).chain(maze.neighbours(i)) {
                        if !std::mem::replace(&mut marked[j], true) {
                            dirty.push(j);
                        }
                    }
                }
                dirty.iter().for_each(|&j| marked[j] = false);
            }
        });
        let x = pool.x.into_inner().unwrap();
        Trace {
            path: read_path(maze, &x),
            loops,
            token_passes,
            reached_at,
        }
    }

    /// Token `i` through the block: `t = x[i] + attn(x)[i]; t += mlp(t)`.
    fn block(&self, i: usize, x: &[Vector], maze: &Maze) -> Vector {
        let nbrs = maze.neighbours(i);
        let mut t = x[i];
        let mut attn = [0.0; D];
        for head in &self.heads {
            // `(bias slot, value)` of every unmasked key; the sink's value is all zero.
            let keys = || {
                let sink = (SINK, 0.0);
                (nbrs.iter().enumerate())
                    .map(|(s, &j)| (s, dot(&head.wv, &x[j])))
                    .chain([sink])
            };
            let max = keys()
                .map(|(s, _)| head.bias[s])
                .fold(f32::NEG_INFINITY, f32::max);
            let (mut total, mut mix) = (0.0, 0.0);
            for (s, v) in keys() {
                let e = (head.bias[s] - max).exp();
                total += e;
                mix += e * v;
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
    pub path: Option<Vec<usize>>,
    /// Loop iterations until the start was reached or the flood stopped growing.
    pub loops: usize,
    /// Token evaluations by the block, summed over all loops; tokens whose
    /// neighbourhood did not change are not re-evaluated.
    pub token_passes: usize,
    /// Per cell id: the loop in which the flood fill from the goals reached it.
    pub reached_at: Vec<Option<usize>>,
}

/// Follows the first goal's pointer chain, then routes through any remaining goals.
fn read_path(maze: &Maze, x: &[Vector]) -> Option<Vec<usize>> {
    let mut cell = maze.start();
    if x[cell][REACHED] != 1.0 {
        return None;
    }
    if maze.goals().len() == 1 {
        let mut path = vec![cell];
        while cell != maze.goal() {
            let slot = (0..K)
                .find(|&k| x[cell][PTR + k] == 1.0)
                .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
            cell = maze.neighbours(cell)[slot];
            path.push(cell);
        }
        return Some(path);
    }

    let n = maze.cells().len();
    let mut unvisited = vec![false; n];
    for &goal in maze.goals() {
        unvisited[goal] = true;
    }
    let mut remaining = maze.goals().len();
    let mut previous = vec![usize::MAX; n];
    let mut queue = VecDeque::with_capacity(n);
    let mut segment = Vec::with_capacity(n);
    let mut path = vec![cell];
    while !unvisited[cell] {
        let slot = (0..K)
            .find(|&k| x[cell][PTR + k] == 1.0)
            .unwrap_or_else(|| panic!("pointer chain broke off after {path:?}"));
        cell = maze.neighbours(cell)[slot];
        path.push(cell);
    }
    unvisited[cell] = false;
    remaining -= 1;
    while remaining > 0 {
        cell = *path.last().unwrap();
        previous.fill(usize::MAX);
        previous[cell] = cell;
        queue.clear();
        queue.push_back(cell);
        let mut target = None;
        while let Some(at) = queue.pop_front() {
            if unvisited[at] {
                target = Some(at);
                break;
            }
            for &next in maze.neighbours(at) {
                if maze.cells()[next] != Cell::Wall && previous[next] == usize::MAX {
                    previous[next] = at;
                    queue.push_back(next);
                }
            }
        }
        let mut at = target?;
        segment.clear();
        segment.push(at);
        while at != cell {
            at = previous[at];
            segment.push(at);
        }
        for &at in segment.iter().rev().skip(1) {
            if unvisited[at] {
                unvisited[at] = false;
                remaining -= 1;
            }
            path.push(at);
        }
    }
    Some(path)
}

/// Token embedding: just the cell type.
fn embed(maze: &Maze) -> Vec<Vector> {
    let token = |&cell: &Cell| {
        let mut t = [0.0; D];
        t[ONE] = 1.0;
        t[OPEN] = f32::from(cell != Cell::Wall);
        t[GOAL] = f32::from(cell == Cell::Goal);
        t
    };
    maze.cells().iter().map(token).collect()
}

/// Head `k` attends to neighbour slot `k`: its bias is 0 there, `-BETA` at the sink, which
/// takes over when slot `k` is missing, and `-2·BETA` at every other slot.
/// It copies the neighbour's `REACHED`.
fn head(k: usize) -> Head {
    let bias = std::array::from_fn(|s| match s {
        _ if s == k => 0.0,
        SINK => -BETA,
        _ => -2.0 * BETA,
    });
    let mut wv = [0.0; D];
    wv[REACHED] = 1.0;
    let mut wo = [0.0; D];
    wo[NBR_REACHED + k] = 1.0;
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
    // reached ← goal ∨ (open ∧ ∃ reached neighbour); `big` outweighs all K neighbours of a wall.
    let big = (K + 1) as f32;
    let any_nbr: Vec<_> = (0..K).map(|d| (NBR_REACHED + d, 1.0)).collect();
    step(
        &[&any_nbr[..], &[(GOAL, big), (OPEN, big)]].concat(),
        -big,
        REACHED,
    );
    // ptr[d] += open ∧ ¬reached ∧ ¬goal ∧ nbr[d] ∧ ¬nbr[e] for e < d (first wins)
    for d in 0..K {
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

/// Worker threads that live for one [`Model::trace`] and evaluate the dirty tokens of
/// each loop, claiming them `CLAIM` at a time.
struct Pool {
    x: RwLock<Vec<Vector>>,
    dirty: RwLock<Vec<usize>>,
    cursor: AtomicUsize,
    /// `(token, new value)` per thread; index 0 is the calling thread.
    out: Vec<Mutex<Vec<(usize, Vector)>>>,
    start: Barrier,
    end: Barrier,
    done: AtomicBool,
    /// First panic caught in `work` this loop, re-raised on the calling thread.
    panic: Mutex<Option<Box<dyn Any + Send>>>,
}

impl Pool {
    /// Blocks a worker until the next loop; `false` once the trace is over.
    fn wait_for_loop(&self) -> bool {
        self.start.wait();
        !self.done.load(Relaxed)
    }

    /// Evaluates claimed tokens until none are left. Never unwinds: a panic is parked in
    /// `panic` so every thread still reaches the `end` barrier instead of deadlocking it.
    fn work(&self, model: &Model, maze: &Maze, id: usize) {
        let run = || self.claim_all(model, maze, id);
        if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(run)) {
            let mut slot = self.panic.lock().unwrap_or_else(PoisonError::into_inner);
            slot.get_or_insert(payload);
        }
    }

    fn claim_all(&self, model: &Model, maze: &Maze, id: usize) {
        let (x, dirty) = (self.x.read().unwrap(), self.dirty.read().unwrap());
        let mut out = self.out[id].lock().unwrap();
        loop {
            let first = self.cursor.fetch_add(CLAIM, Relaxed);
            let Some(ids) = dirty.get(first..(first + CLAIM).min(dirty.len())) else {
                break;
            };
            if ids.is_empty() {
                break;
            }
            out.extend(ids.iter().map(|&i| (i, model.block(i, &x, maze))));
        }
    }
}

/// Wakes the idle workers with `done` set so they exit.
struct Shutdown<'a>(&'a Pool);

impl Drop for Shutdown<'_> {
    fn drop(&mut self) {
        self.0.done.store(true, Relaxed);
        self.0.start.wait();
    }
}
