use labyrinth::Maze;

/// Visits the nearest remaining goal on each leg. Multi-goal benchmarks are trees,
/// so each point-to-point search returns the unique path to its target.
pub(super) fn visit(
    maze: &Maze,
    mut search: impl FnMut(usize, usize) -> Option<Vec<usize>>,
) -> Option<Vec<usize>> {
    let mut from = maze.start();
    if let &[goal] = maze.goals() {
        return search(from, goal);
    }

    let mut remaining = maze.goals().to_vec();
    let mut route = Vec::new();
    while !remaining.is_empty() {
        let mut chosen = 0;
        let mut leg = search(from, remaining[0])?;
        for (i, &goal) in remaining.iter().enumerate().skip(1) {
            let candidate = search(from, goal)?;
            if precedes(maze, &candidate, &leg) {
                chosen = i;
                leg = candidate;
            }
        }
        from = remaining.swap_remove(chosen);
        if route.is_empty() {
            route = leg;
        } else {
            route.extend_from_slice(&leg[1..]);
        }
    }
    Some(route)
}

fn precedes(maze: &Maze, a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return a.len() < b.len();
    }
    // BFS visits the earlier neighbour's subtree first at every given depth.
    for (i, (&left, &right)) in a.iter().zip(b).enumerate().skip(1) {
        if left != right {
            let neighbours = maze.neighbours(a[i - 1]);
            return neighbours.iter().find(|&&n| n == left || n == right) == Some(&left);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::bidirectional_bfs;
    use super::*;
    use labyrinth::Cell::{Goal, Open, Start};

    #[test]
    fn tied_goals_follow_neighbour_order_at_the_fork() {
        let maze = Maze::new(vec![Start, Open, Goal, Goal], [(0, 1), (1, 3), (1, 2)]).unwrap();
        let route = visit(&maze, |from, goal| bidirectional_bfs(&maze, from, goal));
        assert_eq!(route, Some(vec![0, 1, 3, 1, 2]));
    }
}
