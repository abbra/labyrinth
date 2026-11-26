//! A looped transformer with fixed weights for maze solving.

mod block;
#[cfg(target_arch = "x86_64")]
mod cells;

use block::{Block, GOAL, ONE, OPEN, PTR, REACHED, Token};

use crate::{Cell, MAX_DEGREE, Maze, Solver};

/// Solves each leg by evaluating the same attention and ReLU block on every cell.
#[derive(Clone, Debug)]
pub struct Transformer {
    block: Block,
}

/// Evaluates the same fixed weights as [`Transformer`], but only on non-wall cells.
#[derive(Clone, Debug, Default)]
pub struct WallSkippingTransformer(Transformer);

impl WallSkippingTransformer {
    /// Constructs the fixed weights, independent of the maze size and topology.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transformer {
    /// Constructs the fixed weights, independent of the maze size and topology.
    #[must_use]
    pub fn new() -> Self {
        Self {
            block: Block::new(),
        }
    }
}

impl Default for Transformer {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer {
    fn solve_maze<const SKIP_WALLS: bool>(&self, maze: &Maze) -> Option<Vec<usize>> {
        #[cfg(target_arch = "x86_64")]
        if std::is_x86_feature_detected!("avx2") && maze.cells().len() <= i32::MAX as usize {
            // SAFETY: AVX2 is present and even the sentinel fits a signed gather index.
            return unsafe { cells::solve::<SKIP_WALLS>(&self.block, maze) };
        }
        self.solve_portable::<SKIP_WALLS>(maze)
    }

    fn solve_portable<const SKIP_WALLS: bool>(&self, maze: &Maze) -> Option<Vec<usize>> {
        let mut tokens: Vec<_> = maze
            .cells()
            .iter()
            .map(|&cell| {
                let mut token = Token::default();
                token[ONE] = 1.0;
                token[OPEN] = if cell == Cell::Wall { 0.0 } else { 1.0 };
                token[GOAL] = if cell == Cell::Goal { 1.0 } else { 0.0 };
                token
            })
            .collect();
        // Keep original IDs and neighbour slots; both buffers retain immutable wall tokens.
        let traversable: Vec<_> = if SKIP_WALLS {
            (0..maze.cells().len())
                .filter(|&i| maze.cells()[i] != Cell::Wall)
                .collect()
        } else {
            Vec::new()
        };
        let mut next = if SKIP_WALLS {
            tokens.clone()
        } else {
            Vec::with_capacity(tokens.len())
        };
        let mut at = maze.start();
        let mut path = vec![at];
        for leg in 0..maze.goals().len() {
            if leg > 0 {
                // The last goal becomes an ordinary open cell. Re-embed the dynamic channels.
                tokens[at][GOAL] = 0.0;
                for token in &mut tokens {
                    token[REACHED..].fill(0.0);
                }
            }
            loop {
                let mut changed = false;
                let mut evaluate = |i: usize, old: &Token| {
                    let new = self.block.forward(&tokens, maze.neighbours(i), i);
                    changed = changed || new != *old;
                    new
                };
                if SKIP_WALLS {
                    for &i in &traversable {
                        next[i] = evaluate(i, &tokens[i]);
                    }
                } else {
                    next.clear();
                    for (i, old) in tokens.iter().enumerate() {
                        next.push(evaluate(i, old));
                    }
                }
                std::mem::swap(&mut tokens, &mut next);
                if tokens[at][REACHED] > 0.5 {
                    break;
                }
                if !changed {
                    return None;
                }
            }
            while tokens[at][GOAL] < 0.5 {
                let slot = tokens[at][PTR..PTR + MAX_DEGREE]
                    .iter()
                    .position(|&p| p > 0.5)?;
                at = maze.neighbours(at)[slot];
                path.push(at);
            }
        }
        Some(path)
    }
}

impl Solver for Transformer {
    fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        self.solve_maze::<false>(maze)
    }
}

impl Solver for WallSkippingTransformer {
    fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        self.0.solve_maze::<true>(maze)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_wall_skipping_keeps_original_ids_across_legs() {
        let maze: Maze = "#G.S.G#".parse().unwrap();
        let solver = Transformer::new();
        let dense = solver.solve_portable::<false>(&maze);
        assert_eq!(dense, solver.solve_portable::<true>(&maze));
        assert!(dense.is_some());
    }
}
