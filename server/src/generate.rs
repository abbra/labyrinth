//! Random maze generators. Output rows are ASCII art in the format [`labyrinth::Maze`]
//! parses: `#` wall, `.` open, `S` start, `G` goal.

use serde::Deserialize;

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

/// Builds a `height × width` maze (both at least 5) with start and goal in
/// opposite corners.
pub fn generate(algorithm: Algorithm, height: usize, width: usize, seed: u64) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut grid = vec![vec![b'#'; width]; height];
    let (start, goal) = match algorithm {
        Algorithm::Caves => {
            for cell in grid.iter_mut().flatten() {
                if rng.below(100) >= 30 {
                    *cell = b'.';
                }
            }
            ((0, 0), (height - 1, width - 1))
        }
        Algorithm::Backtracker | Algorithm::Braid => {
            backtracker(&mut grid, &mut rng);
            if algorithm == Algorithm::Braid {
                braid(&mut grid, &mut rng);
            }
            ((1, 1), last_room(height, width))
        }
        Algorithm::Prim => {
            prim(&mut grid, &mut rng);
            ((1, 1), last_room(height, width))
        }
    };
    grid[start.0][start.1] = b'S';
    grid[goal.0][goal.1] = b'G';
    grid.into_iter()
        .map(|row| String::from_utf8(row).expect("grid is ASCII"))
        .collect()
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
    use labyrinth::{Maze, Model};

    #[test]
    fn carved_mazes_parse_and_are_solvable() {
        let model = Model::new();
        for algorithm in [Algorithm::Backtracker, Algorithm::Prim, Algorithm::Braid] {
            for (h, w, seed) in [(5, 5, 1), (6, 8, 2), (11, 15, 3), (20, 9, 4)] {
                let rows = generate(algorithm, h, w, seed);
                assert_eq!((rows.len(), rows[0].len()), (h, w));
                let maze: Maze = rows.join("\n").parse().unwrap();
                assert!(
                    model.solve(&maze).is_some(),
                    "{algorithm:?} {h}x{w} unsolvable"
                );
            }
        }
    }

    #[test]
    fn same_seed_same_maze() {
        assert_eq!(
            generate(Algorithm::Caves, 9, 9, 7),
            generate(Algorithm::Caves, 9, 9, 7)
        );
    }
}
