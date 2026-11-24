//! Random maze generators. Output is one byte per board cell in the format
//! [`Board::maze`] reads: `#` wall, `.` open, `S` start, `G` goal.

use serde::Deserialize;

use crate::board::{Board, Shape};

/// How walls are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    /// Depth-first carving: long winding corridors, exactly one path.
    Backtracker,
    /// Randomized Prim: many short dead ends, exactly one path.
    Prim,
    /// Backtracker with most dead ends knocked through: loops and several paths.
    Braid,
    /// Independent random walls: open caverns, sometimes no path at all.
    Caves,
}

/// Fills `board` (square boards at least 5×5) with start near the top-left and goal near
/// the bottom-right.
pub fn generate(board: &Board, algorithm: Algorithm, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    match (algorithm, board.shape) {
        (Algorithm::Caves, _) => {
            let mut cells: Vec<u8> = (0..board.len())
                .map(|_| if rng.below(100) >= 30 { b'.' } else { b'#' })
                .collect();
            let start = corner(board, -1.0, |_| true);
            let goal = corner(board, 1.0, |i| i != start);
            cells[start] = b'S';
            cells[goal] = b'G';
            cells
        }
        (_, Shape::Square) => rooms(algorithm, board.height, board.width, &mut rng),
        _ => carve(board, algorithm, &mut rng),
    }
}

/// The cell furthest along `sign · (x + y)` among those `keep` accepts.
fn corner(board: &Board, sign: f64, keep: impl Fn(usize) -> bool) -> usize {
    let along = |i: usize| {
        let [x, y] = board.centre(i);
        sign * (x + y)
    };
    (0..board.len())
        .filter(|&i| keep(i))
        .max_by(|&a, &b| along(a).total_cmp(&along(b)))
        .expect("board has a free cell")
}

/// Carving on any cell graph: a wall cell is opened only when the passage it extends is
/// its sole open neighbour, so corridors stay one cell wide. Carving grows from the
/// middle, because a corner cell can be boxed in after a handful of steps.
fn carve(board: &Board, algorithm: Algorithm, rng: &mut Rng) -> Vec<u8> {
    let nbrs = &board.neighbours;
    let mut cells = vec![b'#'; board.len()];
    let n = board.len() as f64;
    let [mx, my] = (0..board.len()).fold([0.0, 0.0], |[x, y], i| {
        let [cx, cy] = board.centre(i);
        [x + cx / n, y + cy / n]
    });
    let origin = (0..board.len())
        .min_by(|&a, &b| {
            let d = |i: usize| (board.centre(i)[0] - mx).hypot(board.centre(i)[1] - my);
            d(a).total_cmp(&d(b))
        })
        .expect("board has cells");
    cells[origin] = b'.';
    let carvable = |cells: &[u8], from: usize, to: usize| {
        cells[to] == b'#' && nbrs[to].iter().all(|&k| k == from || cells[k] == b'#')
    };
    if algorithm == Algorithm::Prim {
        let mut frontier: Vec<_> = nbrs[origin].iter().map(|&to| (origin, to)).collect();
        while !frontier.is_empty() {
            let (from, to) = frontier.swap_remove(rng.below(frontier.len()));
            if carvable(&cells, from, to) {
                cells[to] = b'.';
                frontier.extend(nbrs[to].iter().map(|&n| (to, n)));
            }
        }
    } else {
        let mut stack = vec![origin];
        while let Some(&from) = stack.last() {
            let fresh: Vec<_> = nbrs[from]
                .iter()
                .copied()
                .filter(|&to| carvable(&cells, from, to))
                .collect();
            if fresh.is_empty() {
                stack.pop();
            } else {
                let to = fresh[rng.below(fresh.len())];
                cells[to] = b'.';
                stack.push(to);
            }
        }
    }
    if algorithm == Algorithm::Braid {
        // Open one wall of ~70% of dead ends that links them to another passage.
        let open = |cells: &[u8], k: usize| cells[k] != b'#';
        for i in 0..cells.len() {
            let dead_end =
                open(&cells, i) && nbrs[i].iter().filter(|&&k| open(&cells, k)).count() == 1;
            if !dead_end || rng.below(10) >= 7 {
                continue;
            }
            let links: Vec<_> = nbrs[i]
                .iter()
                .copied()
                .filter(|&w| !open(&cells, w) && nbrs[w].iter().any(|&k| k != i && open(&cells, k)))
                .collect();
            if !links.is_empty() {
                cells[links[rng.below(links.len())]] = b'.';
            }
        }
    }
    let start = corner(board, -1.0, |i| cells[i] == b'.');
    let goal = corner(board, 1.0, |i| i != start && cells[i] == b'.');
    cells[start] = b'S';
    cells[goal] = b'G';
    cells
}

/// Classic square-grid carvers that knock out walls between rooms.
fn rooms(algorithm: Algorithm, height: usize, width: usize, rng: &mut Rng) -> Vec<u8> {
    let mut grid = vec![vec![b'#'; width]; height];
    if algorithm == Algorithm::Prim {
        prim(&mut grid, rng);
    } else {
        backtracker(&mut grid, rng);
        if algorithm == Algorithm::Braid {
            braid(&mut grid, rng);
        }
    }
    let goal = last_room(height, width);
    grid[1][1] = b'S';
    grid[goal.0][goal.1] = b'G';
    grid.concat()
}

/// Carving algorithms use "rooms" at odd coordinates and knock out the wall between two
/// rooms to connect them; the outer frame stays wall.
fn last_room(height: usize, width: usize) -> (usize, usize) {
    ((height - 3) | 1, (width - 3) | 1)
}

/// Rooms two steps away from `(r, c)` that lie inside the frame.
fn rooms_around(grid: &[Vec<u8>], (r, c): (usize, usize)) -> impl Iterator<Item = (usize, usize)> {
    let (h, w) = (grid.len(), grid[0].len());
    [(-2, 0), (0, 2), (2, 0), (0, -2)]
        .into_iter()
        .filter_map(move |(dr, dc)| {
            let room = (r.checked_add_signed(dr)?, c.checked_add_signed(dc)?);
            (room.0 >= 1 && room.0 < h - 1 && room.1 >= 1 && room.1 < w - 1).then_some(room)
        })
}

fn connect(grid: &mut [Vec<u8>], a: (usize, usize), b: (usize, usize)) {
    grid[(a.0 + b.0) / 2][(a.1 + b.1) / 2] = b'.';
    grid[b.0][b.1] = b'.';
}

fn backtracker(grid: &mut [Vec<u8>], rng: &mut Rng) {
    grid[1][1] = b'.';
    let mut stack = vec![(1, 1)];
    while let Some(&room) = stack.last() {
        let fresh: Vec<_> = rooms_around(grid, room)
            .filter(|&(r, c)| grid[r][c] == b'#')
            .collect();
        if fresh.is_empty() {
            stack.pop();
        } else {
            let next = fresh[rng.below(fresh.len())];
            connect(grid, room, next);
            stack.push(next);
        }
    }
}

fn prim(grid: &mut [Vec<u8>], rng: &mut Rng) {
    grid[1][1] = b'.';
    let mut frontier: Vec<_> = rooms_around(grid, (1, 1)).collect();
    while !frontier.is_empty() {
        let room = frontier.swap_remove(rng.below(frontier.len()));
        if grid[room.0][room.1] != b'#' {
            continue;
        }
        let carved: Vec<_> = rooms_around(grid, room)
            .filter(|&(r, c)| grid[r][c] == b'.')
            .collect();
        connect(grid, carved[rng.below(carved.len())], room);
        frontier.extend(rooms_around(grid, room).filter(|&(r, c)| grid[r][c] == b'#'));
    }
}

/// Opens one inner wall of roughly 70% of dead ends, turning the tree into a graph.
fn braid(grid: &mut [Vec<u8>], rng: &mut Rng) {
    let (h, w) = (grid.len(), grid[0].len());
    for r in (1..h - 1).step_by(2) {
        for c in (1..w - 1).step_by(2) {
            let walls: Vec<_> = rooms_around(grid, (r, c))
                .map(|room| ((r + room.0) / 2, (c + room.1) / 2))
                .filter(|&(wr, wc)| grid[wr][wc] == b'#')
                .collect();
            let dead_end = grid[r][c] == b'.'
                && !walls.is_empty()
                && walls.len() + 1 == rooms_around(grid, (r, c)).count();
            if dead_end && rng.below(10) < 7 {
                let (wr, wc) = walls[rng.below(walls.len())];
                grid[wr][wc] = b'.';
            }
        }
    }
}

/// xorshift64: tiny, deterministic per seed, good enough for layouts.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform-ish in `0..n`; `n` must be non-zero.
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labyrinth::Model;

    #[test]
    fn carved_mazes_are_solvable_on_every_shape() {
        let model = Model::new();
        for shape in [Shape::Square, Shape::Hex, Shape::Triangle, Shape::Penrose] {
            for algorithm in [Algorithm::Backtracker, Algorithm::Prim, Algorithm::Braid] {
                for (h, w, seed) in [(5, 5, 1), (6, 8, 2), (11, 15, 3), (20, 9, 4)] {
                    let board = Board::new(shape, w, h);
                    let cells = generate(&board, algorithm, seed);
                    let maze = board.maze(&cells).unwrap();
                    assert!(
                        model.solve(&maze).is_some(),
                        "{shape:?} {algorithm:?} {h}x{w} unsolvable"
                    );
                }
            }
        }
    }

    #[test]
    fn same_seed_same_maze() {
        let board = Board::new(Shape::Penrose, 9, 9);
        assert_eq!(
            generate(&board, Algorithm::Braid, 7),
            generate(&board, Algorithm::Braid, 7)
        );
    }

    #[test]
    fn large_boards_are_carved_throughout() {
        // Penrose 321 seed 1 once boxed its corner start in after 19 cells.
        for shape in [Shape::Hex, Shape::Triangle, Shape::Penrose] {
            let board = Board::new(shape, 321, 321);
            let cells = generate(&board, Algorithm::Backtracker, 1);
            let open = cells.iter().filter(|&&c| c != b'#').count();
            assert!(
                open * 3 > cells.len(),
                "{shape:?}: {open} of {} open",
                cells.len()
            );
        }
    }
}
