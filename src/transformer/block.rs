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

// One row per key slot, with the head dimension contiguous for vectorization.
type HeadRow = [f32; MAX_DEGREE];

#[derive(Clone, Debug)]
pub(super) struct Block {
    probabilities: [[HeadRow; MAX_DEGREE]; MAX_DEGREE + 1],
}

// The two ReLU units for a threshold share the same input projection.
#[inline]
fn step(projected: f32, one: f32) -> f32 {
    (projected - 4.5 * one).max(0.0) - (projected - 5.5 * one).max(0.0)
}

impl Block {
    pub(super) fn new() -> Self {
        let sink = (-25.0_f32).exp();
        let other = (-50.0_f32).exp();
        let probabilities = array::from_fn(|degree| {
            array::from_fn(|slot| {
                array::from_fn(|head| {
                    let total = if head < degree {
                        1.0 + sink + (degree - 1) as f32 * other
                    } else {
                        sink + degree as f32 * other
                    };
                    (if head == slot { 1.0 } else { other }) / total
                })
            })
        });
        Self { probabilities }
    }

    fn attend(&self, input: &[Token], neighbours: &[usize]) -> HeadRow {
        let mut heads = [0.0; MAX_DEGREE];
        for (&key, weights) in neighbours.iter().zip(&self.probabilities[neighbours.len()]) {
            let value = input[key][REACHED];
            for (head, weight) in heads.iter_mut().zip(weights) {
                *head += weight * value;
            }
        }
        heads
    }

    pub(super) fn forward(&self, input: &[Token], neighbours: &[usize], i: usize) -> Token {
        let mut output = input[i];
        let [one, open, goal, reached, ..] = output;
        let ptr_base = 10.0 * (open - one - goal - reached);
        let mut prefix = 0.0;
        for (slot, value) in self.attend(input, neighbours).into_iter().enumerate() {
            let scratch = output[NBR_REACHED + slot] + value;
            let scaled = 10.0 * scratch;
            // Pointer rows differ only in their selected slot and the negative prefix.
            output[PTR + slot] += step(ptr_base - prefix + scaled, one);
            prefix += scaled;
            output[NBR_REACHED + slot] = scratch - scratch.max(0.0);
        }
        let reach = (10 * MAX_DEGREE) as f32 * (open - one) + 10.0 * goal + prefix;
        output[REACHED] += step(reach, one) - reached.max(0.0);
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
        let leak = block.attend(&input, &[1])[1];
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
