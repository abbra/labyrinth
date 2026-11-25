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

    /// # Safety
    /// The caller must verify AVX2 support.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub(super) fn forward_avx2(&self, input: &[Token], neighbours: &[usize], i: usize) -> Token {
        use std::arch::x86_64::*;

        // Each lane corresponds to a single attention head.
        const _: () = assert!(MAX_DEGREE == 8);
        // SAFETY: each unaligned load/store spans exactly one eight-float row.
        let heads = unsafe {
            let mut sum = _mm256_setzero_ps();
            for (&key, row) in neighbours.iter().zip(&self.probabilities[neighbours.len()]) {
                let value = _mm256_set1_ps(input[key][REACHED]);
                sum = _mm256_add_ps(sum, _mm256_mul_ps(value, _mm256_loadu_ps(row.as_ptr())));
            }
            let mut heads = [0.0; MAX_DEGREE];
            _mm256_storeu_ps(heads.as_mut_ptr(), sum);
            heads
        };
        Self::update(input[i], heads)
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
        Self::update(input[i], self.attend(input, neighbours))
    }

    #[inline(always)]
    fn update(mut output: Token, heads: HeadRow) -> Token {
        let [one, open, goal, reached, ..] = output;
        let ptr_base = 10.0 * (open - one - goal - reached);
        let scratch: HeadRow = array::from_fn(|d| output[NBR_REACHED + d] + heads[d]);
        let scaled = scratch.map(|v| 10.0 * v);
        // Keep the ordered f32 prefix separate so LLVM can vectorize the eight ReLU pairs.
        let mut prefix = 0.0;
        let earlier = scaled.map(|v| {
            let previous = prefix;
            prefix += v;
            previous
        });
        for d in 0..MAX_DEGREE {
            output[PTR + d] += step(ptr_base - earlier[d] + scaled[d], one);
            output[NBR_REACHED + d] = scratch[d] - scratch[d].max(0.0);
        }
        let reach = (10 * MAX_DEGREE) as f32 * (open - one) + 10.0 * goal + prefix;
        output[REACHED] += step(reach, one) - reached.max(0.0);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn avx2_matches_portable_for_every_degree() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }
        let block = Block::new();
        let mut input = [[0.0; WIDTH]; MAX_DEGREE + 1];
        input[0][ONE] = 1.0;
        input[0][OPEN] = 1.0;
        let neighbours: [usize; MAX_DEGREE] = array::from_fn(|i| i + 1);
        for active in neighbours {
            input[active][REACHED] = 0.5;
            for degree in 0..=MAX_DEGREE {
                let keys = &neighbours[..degree];
                // SAFETY: AVX2 support was checked above.
                let actual = unsafe { block.forward_avx2(&input, keys, 0) };
                assert_eq!(actual, block.forward(&input, keys, 0));
            }
            input[active][REACHED] = 0.0;
        }
    }

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
