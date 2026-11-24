use std::collections::VecDeque;

use labyrinth::{Cell, MAX_DEGREE, Maze, Model};

/// Reference shortest-path length (in cells) by plain BFS.
fn bfs_len(maze: &Maze) -> Option<usize> {
    let mut dist = vec![usize::MAX; maze.cells().len()];
    let mut queue = VecDeque::from([maze.start()]);
    dist[maze.start()] = 1;
    while let Some(c) = queue.pop_front() {
        if c == maze.goal() {
            return Some(dist[c]);
        }
        for &n in maze.neighbours(c) {
            if maze.cells()[n] != Cell::Wall && dist[n] == usize::MAX {
                dist[n] = dist[c] + 1;
                queue.push_back(n);
            }
        }
    }
    None
}

/// Solves `maze` with the model and checks the path against BFS; returns whether solvable.
fn check(maze: &Maze, what: &str) -> bool {
    let path = Model::new().solve(maze);
    assert_eq!(
        path.as_ref().map(Vec::len),
        bfs_len(maze),
        "{what}: {maze:?}"
    );
    let Some(path) = path else { return false };
    assert_eq!((path[0], path[path.len() - 1]), (maze.start(), maze.goal()));
    for pair in path.windows(2) {
        assert!(
            maze.neighbours(pair[0]).contains(&pair[1]),
            "{what}: jump {pair:?}"
        );
        assert_ne!(maze.cells()[pair[1]], Cell::Wall, "{what}: through wall");
    }
    true
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// ~35% walls, S and G at random distinct cells.
fn random_cells(rng: &mut Rng, n: usize) -> Vec<Cell> {
    let mut cells: Vec<_> = (0..n)
        .map(|_| {
            if rng.below(100) < 35 {
                Cell::Wall
            } else {
                Cell::Open
            }
        })
        .collect();
    let start = rng.below(n);
    let goal = (start + 1 + rng.below(n - 1)) % n;
    cells[start] = Cell::Start;
    cells[goal] = Cell::Goal;
    cells
}

/// Edges of an `h × w` hex grid in "odd rows shifted right" offset layout.
fn hex_edges(h: usize, w: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            if c + 1 < w {
                edges.push((i, i + 1));
            }
            if r + 1 < h {
                // Down-left and down-right neighbours.
                let (left, right) = if r % 2 == 0 {
                    (c.checked_sub(1), Some(c))
                } else {
                    (Some(c), Some(c + 1))
                };
                for d in [left, right].into_iter().flatten().filter(|&d| d < w) {
                    edges.push((i, i + w + d - c));
                }
            }
        }
    }
    edges
}

/// Edges of an `h × w` triangle grid: a triangle points up when `r + c` is even and then
/// shares its base with the one below.
fn triangle_edges(h: usize, w: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            if c + 1 < w {
                edges.push((i, i + 1));
            }
            if r + 1 < h && (r + c) % 2 == 1 {
                edges.push((i, i + w));
            }
        }
    }
    edges
}

/// Random graph of irregular degree up to `MAX_DEGREE`, like an aperiodic tiling.
fn irregular_edges(rng: &mut Rng, n: usize) -> Vec<(usize, usize)> {
    let mut degree = vec![0; n];
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for _ in 0..2 * n {
        let a = rng.below(n);
        // Mostly local edges, so paths are long.
        let b = (a + 1 + rng.below(4)) % n;
        if a == b
            || degree[a] == MAX_DEGREE
            || degree[b] == MAX_DEGREE
            || edges.contains(&(a.min(b), a.max(b)))
        {
            continue;
        }
        degree[a] += 1;
        degree[b] += 1;
        edges.push((a.min(b), a.max(b)));
    }
    edges
}

#[test]
fn matches_bfs_on_random_square_mazes() {
    let mut solved = 0;
    for seed in 0..150 {
        let (h, w) = (2 + seed as usize % 7, 2 + seed as usize / 7 % 9);
        let mut rng = Rng::new(seed);
        let cells = random_cells(&mut rng, h * w);
        let text: Vec<String> = cells
            .chunks(w)
            .map(|row| {
                row.iter()
                    .map(|c| match c {
                        Cell::Wall => '#',
                        Cell::Open => '.',
                        Cell::Start => 'S',
                        Cell::Goal => 'G',
                    })
                    .collect()
            })
            .collect();
        let maze: Maze = text.join("\n").parse().expect("well-formed");
        solved += usize::from(check(&maze, &format!("square seed {seed}")));
    }
    assert!(solved > 50, "too few solvable samples: {solved}");
}

#[test]
fn matches_bfs_on_hex_triangle_and_irregular_mazes() {
    let mut solved = [0; 3];
    for seed in 0..150 {
        let (h, w) = (2 + seed as usize % 7, 2 + seed as usize / 7 % 9);
        let mut rng = Rng::new(seed);
        let shapes = [
            ("hex", hex_edges(h, w)),
            ("triangle", triangle_edges(h, w)),
            ("irregular", irregular_edges(&mut rng, h * w)),
        ];
        for (k, (name, edges)) in shapes.into_iter().enumerate() {
            let maze = Maze::new(random_cells(&mut rng, h * w), edges).expect("valid graph");
            solved[k] += usize::from(check(&maze, &format!("{name} seed {seed}")));
        }
    }
    assert!(
        solved.iter().all(|&s| s > 30),
        "too few solvable samples: {solved:?}"
    );
}

#[test]
fn walls_stay_walls_at_max_degree() {
    // A wall whose every neighbour is reached must not be flooded.
    let mut cells = vec![Cell::Open; MAX_DEGREE + 1];
    cells[0] = Cell::Wall;
    cells[1] = Cell::Goal;
    cells[2] = Cell::Start;
    let ring = (1..MAX_DEGREE).map(|i| (i, i + 1));
    let maze = Maze::new(cells, (1..=MAX_DEGREE).map(|i| (0, i)).chain(ring)).expect("valid");
    let trace = Model::new().trace(&maze);
    assert_eq!(trace.reached_at[0], None);
    assert_eq!(trace.path, Some(vec![2, 1]));
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
    assert!(check(&maze, "corridor"));
}

#[test]
fn visits_every_goal_and_fails_if_any_goal_is_unreachable() {
    let maze: Maze = "S.G.G".parse().expect("multiple goals are valid");
    assert_eq!(maze.goals(), [2, 4]);
    assert_eq!(Model::new().solve(&maze), Some(vec![0, 1, 2, 3, 4]));

    let maze = Maze::new(
        vec![Cell::Start, Cell::Goal, Cell::Open, Cell::Goal],
        [(0, 1), (0, 2), (2, 3)],
    )
    .expect("valid connected maze");
    assert_eq!(
        Model::new().solve(&maze),
        Some(vec![0, 1, 0, 2, 3]),
        "the route may revisit the start to reach another goal"
    );

    let maze: Maze = "S.G#G".parse().expect("multiple goals are valid");
    assert_eq!(Model::new().solve(&maze), None);
}

#[test]
fn solve_rule_frontier_matches_trace_for_single_and_multiple_goals() {
    let model = Model::new();
    let single: Maze = "S..\n...\n..G".parse().expect("valid maze");
    assert_eq!(model.solve(&single), model.trace(&single).path);

    let incomplete_square = Maze::new(
        vec![
            Cell::Start,
            Cell::Open,
            Cell::Goal,
            Cell::Open,
            Cell::Open,
            Cell::Open,
            Cell::Open,
            Cell::Open,
            Cell::Open,
        ],
        [
            (1, 2),
            (3, 4),
            (4, 5),
            (6, 7),
            (7, 8),
            (0, 3),
            (1, 4),
            (2, 5),
            (3, 6),
            (4, 7),
            (5, 8),
        ],
    )
    .expect("valid incomplete square");
    assert_eq!(
        model.solve(&incomplete_square),
        model.trace(&incomplete_square).path
    );

    let multiple = Maze::new(
        vec![
            Cell::Goal,
            Cell::Open,
            Cell::Open,
            Cell::Open,
            Cell::Start,
            Cell::Goal,
        ],
        [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)],
    )
    .expect("valid maze");
    let path = Some(vec![4, 5, 4, 3, 2, 1, 0]);
    assert_eq!(model.solve(&multiple), path);
    assert_eq!(model.solve(&multiple), model.trace(&multiple).path);
}
