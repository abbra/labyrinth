use super::block::Block;
use crate::{Cell, MAX_DEGREE, Maze};
use std::arch::x86_64::*;
use std::array;

const LANES: usize = 8;

struct Slot {
    keys: __m256i,
    weights: [__m256; MAX_DEGREE],
}

/// # Safety
/// Requires AVX2. The caller limits cell count to the signed gather index range.
#[target_feature(enable = "avx2")]
pub(super) fn solve(block: &Block, maze: &Maze) -> Option<Vec<usize>> {
    let pack = |v: [f32; LANES]| {
        // SAFETY: v contains eight readable floats.
        unsafe { _mm256_loadu_ps(v.as_ptr()) }
    };
    let n = maze.cells().len();
    let batches = n.div_ceil(LANES);
    let zero = _mm256_setzero_ps();
    let one = _mm256_set1_ps(1.0);
    let ten = _mm256_set1_ps(10.0);
    let step = |x| {
        _mm256_sub_ps(
            _mm256_max_ps(_mm256_sub_ps(x, _mm256_set1_ps(4.5)), zero),
            _mm256_max_ps(_mm256_sub_ps(x, _mm256_set1_ps(5.5)), zero),
        )
    };
    let mut slots = Vec::new();
    let mut ends = vec![0];
    let mut open = Vec::with_capacity(batches);
    let mut goal = vec![[0.0; LANES]; batches];
    for (b, cells) in maze.cells().chunks(LANES).enumerate() {
        let neighbours: [&[usize]; LANES] = array::from_fn(|l| {
            if l < cells.len() {
                maze.neighbours(b * LANES + l)
            } else {
                &[]
            }
        });
        open.push(pack(array::from_fn(|l| {
            f32::from(cells.get(l).is_some_and(|c| *c != Cell::Wall))
        })));
        for (l, cell) in cells.iter().enumerate() {
            goal[b][l] = f32::from(*cell == Cell::Goal);
        }
        let longest = neighbours
            .iter()
            .copied()
            .max_by_key(|s| s.len())
            .unwrap_or(&[]);
        for (k, _) in longest.iter().enumerate() {
            slots.push(Slot {
                // SAFETY: the index array spans exactly one unaligned vector.
                keys: unsafe {
                    _mm256_loadu_si256(
                        array::from_fn::<_, LANES, _>(|l| {
                            neighbours[l].get(k).copied().unwrap_or(n) as i32
                        })
                        .as_ptr()
                        .cast(),
                    )
                },
                weights: array::from_fn(|h| {
                    pack(array::from_fn(|l| {
                        block.probabilities[neighbours[l].len()][k][h]
                    }))
                }),
            });
        }
        ends.push(slots.len());
    }
    // The last element (or the first padding lane) supplies zero for absent keys.
    let mut reached = vec![0.0; batches * LANES + 1];
    let mut next = reached.clone();
    let mut pointers = vec![[zero; MAX_DEGREE]; batches];
    let mut at = maze.start();
    let mut path = vec![at];
    for leg in 0..maze.goals().len() {
        if leg > 0 {
            goal[at / LANES][at % LANES] = 0.0;
            reached.fill(0.0);
            pointers.fill([zero; MAX_DEGREE]);
        }
        loop {
            let mut changed = zero;
            for b in 0..batches {
                let mut heads = [zero; MAX_DEGREE];
                for slot in &slots[ends[b]..ends[b + 1]] {
                    // SAFETY: each index is a maze neighbour or n, both within reached.
                    let values = unsafe { _mm256_i32gather_ps::<4>(reached.as_ptr(), slot.keys) };
                    for (head, weight) in heads.iter_mut().zip(slot.weights) {
                        *head = _mm256_add_ps(*head, _mm256_mul_ps(values, weight));
                    }
                }
                // SAFETY: both buffers contain LANES entries per batch, including padding.
                let old = unsafe { _mm256_loadu_ps(reached.as_ptr().add(b * LANES)) };
                let g = pack(goal[b]);
                let gate = _mm256_sub_ps(open[b], one);
                let base = _mm256_mul_ps(ten, _mm256_sub_ps(_mm256_sub_ps(gate, g), old));
                let mut prefix = zero;
                for (ptr, head) in pointers[b].iter_mut().zip(heads) {
                    let scaled = _mm256_mul_ps(ten, head);
                    let delta = step(_mm256_add_ps(_mm256_sub_ps(base, prefix), scaled));
                    *ptr = _mm256_add_ps(*ptr, delta);
                    prefix = _mm256_add_ps(prefix, scaled);
                }
                // Nonnegative scratch and old reached are cleared by their residual ReLUs.
                let projection = _mm256_add_ps(
                    _mm256_mul_ps(_mm256_set1_ps((10 * MAX_DEGREE) as f32), gate),
                    _mm256_mul_ps(ten, g),
                );
                let new = step(_mm256_add_ps(projection, prefix));
                changed = _mm256_or_ps(changed, _mm256_cmp_ps::<_CMP_NEQ_OQ>(old, new));
                // SAFETY: this batch has eight writable entries, including any padding.
                unsafe { _mm256_storeu_ps(next.as_mut_ptr().add(b * LANES), new) };
            }
            std::mem::swap(&mut reached, &mut next);
            if reached[at] > 0.5 {
                break;
            }
            if _mm256_movemask_ps(changed) == 0 {
                return None;
            }
        }
        while goal[at / LANES][at % LANES] < 0.5 {
            let k = pointers[at / LANES].iter().position(|p| {
                _mm256_movemask_ps(_mm256_cmp_ps::<_CMP_GT_OQ>(*p, _mm256_set1_ps(0.5)))
                    & (1 << (at % LANES))
                    != 0
            })?;
            at = maze.neighbours(at)[k];
            path.push(at);
        }
    }
    Some(path)
}
