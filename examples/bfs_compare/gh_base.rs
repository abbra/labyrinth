//! Gholami–Hajiaghayi Algorithm 8, lines 3–6, for simple unit-edge mazes.
use std::collections::VecDeque;

use labyrinth::{Cell, Maze};

fn source_distances(maze: &Maze, source: usize) -> Vec<usize> {
    let mut distance = vec![usize::MAX; maze.cells().len()];
    distance[source] = 0;
    let mut pending = vec![source];
    while !pending.is_empty() {
        // Scan finite unsettled labels for the minimum; no heap or FIFO ordering.
        let min = (0..pending.len())
            .min_by_key(|&i| distance[pending[i]])
            .unwrap();
        let cell = pending.swap_remove(min);
        let offer = distance[cell] + 1;
        for &next in maze.neighbours(cell) {
            if maze.cells()[next] != Cell::Wall && offer < distance[next] {
                if distance[next] == usize::MAX {
                    pending.push(next);
                }
                distance[next] = offer;
            }
        }
    }
    distance
}

pub(super) fn solve(maze: &Maze) -> Option<Vec<usize>> {
    let mut goals = vec![false; maze.cells().len()];
    for &goal in maze.goals() {
        goals[goal] = true;
    }
    let mut route = vec![maze.start()];
    let mut source = maze.start();
    for _ in maze.goals() {
        let distance = source_distances(maze, source);
        let mut parent = vec![usize::MAX; distance.len()];
        parent[source] = source;
        let mut queue = VecDeque::from([source]);
        let mut target = None;
        while let Some(cell) = queue.pop_front() {
            if target.is_none() && goals[cell] {
                target = Some(cell);
            }
            // Finish the tight-edge tree even after finding the leg's target.
            for &next in maze.neighbours(cell) {
                if parent[next] == usize::MAX && distance[next] == distance[cell] + 1 {
                    parent[next] = cell;
                    queue.push_back(next);
                }
            }
        }
        let goal = target?;
        let leg = route.len();
        let mut cell = goal;
        while cell != source {
            route.push(cell);
            cell = parent[cell];
        }
        route[leg..].reverse();
        goals[goal] = false;
        source = goal;
    }
    Some(route)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{nearest_goal_bfs, square_maze};

    #[test]
    fn tied_targets_and_tied_prefixes() {
        let maze = square_maze(3, b"G...S...G");
        assert_eq!(solve(&maze), nearest_goal_bfs(&maze));
    }

    #[test]
    fn labels_beyond_goal_and_disconnected_goal() {
        let maze = square_maze(3, b"SG.###..G");
        assert_eq!(&source_distances(&maze, 0)[..3], &[0, 1, 2]);
        assert_eq!(solve(&maze), None);
    }
}
