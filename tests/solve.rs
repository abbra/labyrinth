use std::collections::VecDeque;

use labyrinth::{Cell, Maze, Model, Pos};

/// Reference shortest-path length (in cells) by plain BFS.
fn bfs_len(maze: &Maze) -> Option<usize> {
    let mut dist = vec![usize::MAX; maze.width() * maze.height()];
    let idx = |p: Pos| p.row * maze.width() + p.col;
    let mut queue = VecDeque::from([maze.start()]);
    dist[idx(maze.start())] = 1;
    while let Some(p) = queue.pop_front() {
        if p == maze.goal() {
            return Some(dist[idx(p)]);
        }
        for n in neighbours(p) {
            if matches!(maze.cell(n), Some(c) if c != Cell::Wall) && dist[idx(n)] == usize::MAX {
                dist[idx(n)] = dist[idx(p)] + 1;
                queue.push_back(n);
            }
        }
    }
    None
}

fn neighbours(p: Pos) -> impl Iterator<Item = Pos> {
    [(-1, 0), (0, 1), (1, 0), (0, -1)]
        .into_iter()
        .filter_map(move |(dr, dc)| {
            Some(Pos {
                row: p.row.checked_add_signed(dr)?,
                col: p.col.checked_add_signed(dc)?,
            })
        })
}

/// Deterministic random mazes: ~35% walls, S and G at random distinct cells.
fn random_maze(seed: u64, h: usize, w: usize) -> Maze {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut rng = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut grid: Vec<Vec<char>> = (0..h)
        .map(|_| {
            (0..w)
                .map(|_| if rng() % 100 < 35 { '#' } else { '.' })
                .collect()
        })
        .collect();
    let start = (rng() as usize % h, rng() as usize % w);
    let goal = loop {
        let g = (rng() as usize % h, rng() as usize % w);
        if g != start {
            break g;
        }
    };
    grid[start.0][start.1] = 'S';
    grid[goal.0][goal.1] = 'G';
    let text: Vec<String> = grid.into_iter().map(String::from_iter).collect();
    text.join("\n")
        .parse()
        .expect("generated maze is well-formed")
}

#[test]
fn matches_bfs_on_random_mazes() {
    let model = Model::new();
    let mut solved = 0;
    for seed in 0..150 {
        let (h, w) = (2 + seed as usize % 7, 2 + seed as usize / 7 % 9);
        let maze = random_maze(seed, h, w);
        let path = model.solve(&maze);
        assert_eq!(
            path.as_ref().map(Vec::len),
            bfs_len(&maze),
            "seed {seed}: {maze:?}"
        );
        let Some(path) = path else { continue };
        solved += 1;
        assert_eq!((path[0], path[path.len() - 1]), (maze.start(), maze.goal()));
        for pair in path.windows(2) {
            assert!(
                neighbours(pair[0]).any(|n| n == pair[1]),
                "seed {seed}: jump {pair:?}"
            );
            assert_ne!(
                maze.cell(pair[1]),
                Some(Cell::Wall),
                "seed {seed}: through wall"
            );
        }
    }
    assert!(solved > 50, "too few solvable samples: {solved}");
}

#[test]
fn long_corridor_maze() {
    let maze: Maze = "\
S.#.......
#.#.#####.
#...#...#.
#####.#.#.
#.....#...
#.########
#........G"
        .parse()
        .expect("valid maze");
    let path = Model::new().solve(&maze).expect("solvable");
    assert_eq!(Some(path.len()), bfs_len(&maze));
}
