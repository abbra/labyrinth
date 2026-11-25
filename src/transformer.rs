//! A looped transformer with fixed weights for maze solving.

mod block;

use block::{Block, GOAL, ONE, OPEN, PTR, REACHED, Token};

use crate::{Cell, MAX_DEGREE, Maze, Solver};

/// Solves each leg by evaluating the same attention and ReLU block on every cell.
#[derive(Clone, Debug)]
pub struct Transformer {
    block: Block,
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

impl Solver for Transformer {
    fn solve(&self, maze: &Maze) -> Option<Vec<usize>> {
        #[cfg(target_arch = "x86_64")]
        if std::is_x86_feature_detected!("avx2") {
            return self.solve_with(maze, |block, input, neighbours, i| {
                // SAFETY: AVX2 was detected before entering the solve loop.
                unsafe { block.forward_avx2(input, neighbours, i) }
            });
        }
        self.solve_with(maze, Block::forward)
    }
}

impl Transformer {
    fn solve_with(
        &self,
        maze: &Maze,
        forward: impl Fn(&Block, &[Token], &[usize], usize) -> Token,
    ) -> Option<Vec<usize>> {
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
        let mut next = Vec::with_capacity(tokens.len());
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
                next.clear();
                let mut changed = false;
                for (i, old) in tokens.iter().enumerate() {
                    let new = forward(&self.block, &tokens, maze.neighbours(i), i);
                    changed = changed || new != *old;
                    next.push(new);
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
