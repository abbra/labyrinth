use crate::MAX_DEGREE;
use std::array;

pub(super) const ONE: usize = 0;
pub(super) const OPEN: usize = 1;
pub(super) const GOAL: usize = 2;
pub(super) const REACHED: usize = 3;
pub(super) const PTR: usize = 4;
const NBR_REACHED: usize = PTR + MAX_DEGREE;
const WIDTH: usize = NBR_REACHED + MAX_DEGREE;
pub(super) type Token = [f32; WIDTH];

// Each value and output projection has a single nonzero coefficient.
#[derive(Clone, Debug)]
struct Head {
    value: (usize, f32),
    output: (usize, f32),
    probabilities: [[f32; MAX_DEGREE]; MAX_DEGREE + 1],
}

impl Head {
    fn new(slot: usize) -> Self {
        let mut bias = [-50.0_f32; MAX_DEGREE + 1];
        bias[slot] = 0.0;
        bias[MAX_DEGREE] = -25.0; // Sink: counted in softmax, but its value is zero.
        let mass = bias.map(f32::exp);
        let probabilities = array::from_fn(|degree| {
            let total = mass[..degree].iter().sum::<f32>() + mass[MAX_DEGREE];
            let mut row = [0.0; MAX_DEGREE];
            for (weight, &mass) in row[..degree].iter_mut().zip(&mass) {
                *weight = mass / total;
            }
            row
        });
        Self {
            value: (REACHED, 1.0),
            output: (NBR_REACHED + slot, 1.0),
            probabilities,
        }
    }

    fn attend(&self, input: &[Token], neighbours: &[usize]) -> f32 {
        let (channel, gain) = self.value;
        let weights = &self.probabilities[neighbours.len()];
        gain * neighbours
            .iter()
            .zip(weights)
            .map(|(&key, &w)| w * input[key][channel])
            .sum::<f32>()
    }
}

const STEPS: usize = 1 + MAX_DEGREE; // Reached, then one pointer per slot.
const HIDDEN: usize = 2 * STEPS + 1 + MAX_DEGREE;

#[derive(Clone, Debug)]
struct Unit {
    input: Token,
    nonzero: [u8; WIDTH],
    len: usize,
    output: (usize, f32),
}

impl Unit {
    fn new(i: usize) -> Self {
        let mut input = [0.0; WIDTH];
        let output = if i < 2 * STEPS {
            // The pair computes ReLU(10z - 4.5) - ReLU(10z - 5.5).
            let step = i / 2;
            let target = if step == 0 {
                // z = sum(neighbours) + degree_limit * (open - one) + goal.
                input[OPEN] = 10.0 * MAX_DEGREE as f32;
                input[ONE] = -input[OPEN];
                input[GOAL] = 10.0;
                input[NBR_REACHED..].fill(10.0);
                REACHED
            } else {
                // z = open + neighbour[d] - one - goal - reached - sum(earlier).
                let d = step - 1;
                input[OPEN] = 10.0;
                input[ONE] = -10.0;
                input[GOAL] = -10.0;
                input[REACHED] = -10.0;
                input[NBR_REACHED..NBR_REACHED + d].fill(-10.0);
                input[NBR_REACHED + d] = 10.0;
                PTR + d
            };
            if i.is_multiple_of(2) {
                input[ONE] -= 4.5;
                (target, 1.0)
            } else {
                input[ONE] -= 5.5;
                (target, -1.0)
            }
        } else {
            // Subtract old reached and each scratch channel from the residual.
            let clear = i - 2 * STEPS;
            let target = if clear == 0 {
                REACHED
            } else {
                NBR_REACHED + clear - 1
            };
            input[target] = 1.0;
            (target, -1.0)
        };
        let mut nonzero = [0; WIDTH];
        let mut len = 0;
        for (channel, &weight) in input.iter().enumerate() {
            if weight != 0.0 {
                nonzero[len] = channel as u8;
                len += 1;
            }
        }
        Self {
            input,
            nonzero,
            len,
            output,
        }
    }

    fn activate(&self, token: &Token) -> f32 {
        self.nonzero[..self.len]
            .iter()
            .map(|&c| {
                let c = usize::from(c);
                self.input[c] * token[c]
            })
            .sum::<f32>()
            .max(0.0)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Block {
    heads: [Head; MAX_DEGREE],
    mlp: [Unit; HIDDEN],
}

impl Block {
    pub(super) fn new() -> Self {
        Self {
            heads: array::from_fn(Head::new),
            mlp: array::from_fn(Unit::new),
        }
    }

    pub(super) fn forward(&self, input: &[Token], neighbours: &[usize], i: usize) -> Token {
        let mut attended = input[i];
        for head in &self.heads {
            let (channel, gain) = head.output;
            attended[channel] += gain * head.attend(input, neighbours);
        }
        let mut output = attended;
        for unit in &self.mlp {
            let (channel, gain) = unit.output;
            output[channel] += gain * unit.activate(&attended);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_values_follow_the_relu_ramp_and_clear_scratch() {
        let block = Block::new();
        let mut input = [[0.0; WIDTH]; 2];
        input[0][ONE] = 1.0;
        input[0][OPEN] = 1.0;
        input[1][REACHED] = 0.5;
        let leak = block.heads[1].attend(&input, &[1]);
        assert!(leak > 0.0 && leak < f32::EPSILON);
        let output = block.forward(&input, &[1], 0);
        assert_eq!(output[REACHED], 0.5);
        assert_eq!(output[PTR], 0.5);
        assert_eq!(output[NBR_REACHED..], [0.0; MAX_DEGREE]);
    }

    #[test]
    fn even_a_wall_surrounded_by_reached_cells_stays_unreached() {
        let mut input = [[0.0; WIDTH]; 1 + MAX_DEGREE];
        input[0][ONE] = 1.0;
        for token in &mut input[1..] {
            token[REACHED] = 1.0;
        }
        let neighbours: [usize; MAX_DEGREE] = array::from_fn(|i| i + 1);
        assert_eq!(Block::new().forward(&input, &neighbours, 0), input[0]);
    }
}
