use labyrinth::{Cell, MAX_DEGREE, Maze, Model, Solver, Transformer, WallSkippingTransformer};

fn check(maze: &Maze, expected: Option<&[usize]>) {
    let route = Transformer::new().solve(maze);
    assert_eq!(route.as_deref(), expected);
    assert_eq!(route, WallSkippingTransformer::new().solve(maze));
    assert_eq!(route, Model::new().solve(maze));
}

#[test]
fn lowest_slot_breaks_shortest_path_ties() {
    // The goal's first neighbour is 1, but the start's first neighbour is 2.
    let maze = Maze::new(
        vec![Cell::Start, Cell::Open, Cell::Open, Cell::Goal],
        [(1, 3), (0, 2), (0, 1), (2, 3)],
    )
    .unwrap();
    check(&maze, Some(&[0, 2, 3]));
}

#[test]
fn nearest_unvisited_goal_ties_follow_slot_order_on_every_leg() {
    let maze = Maze::new(
        vec![Cell::Start, Cell::Goal, Cell::Goal, Cell::Goal],
        [(0, 3), (0, 2), (0, 1)],
    )
    .unwrap();
    check(&maze, Some(&[0, 3, 0, 2, 0, 1]));
}

#[test]
fn later_legs_can_cross_visited_goals() {
    let maze: Maze = "G.GS.G".parse().unwrap();
    check(&maze, Some(&[3, 2, 1, 0, 1, 2, 3, 4, 5]));
}

#[test]
fn disconnected_later_goal_discards_the_partial_route() {
    let maze: Maze = "S.G#G".parse().unwrap();
    check(&maze, None);
}

#[test]
fn eighth_neighbour_can_supply_the_pointer() {
    let mut cells = vec![Cell::Open; MAX_DEGREE + 1];
    cells[0] = Cell::Start;
    cells[MAX_DEGREE] = Cell::Goal;
    let maze = Maze::new(cells, (1..=MAX_DEGREE).map(|i| (0, i))).unwrap();
    check(&maze, Some(&[0, MAX_DEGREE]));
}

#[test]
fn wall_slots_preserve_tie_order_and_original_route_ids() {
    let maze = Maze::new(
        vec![Cell::Wall, Cell::Goal, Cell::Start, Cell::Wall, Cell::Goal],
        [(2, 0), (2, 4), (2, 3), (2, 1)],
    )
    .unwrap();
    check(&maze, Some(&[2, 4, 2, 1]));
}

#[test]
fn max_degree_wall_blocks_even_several_reached_neighbours() {
    let mut cells = vec![Cell::Goal; MAX_DEGREE + 1];
    cells[0] = Cell::Wall;
    cells[1] = Cell::Start;
    let maze = Maze::new(cells, (1..=MAX_DEGREE).map(|i| (0, i))).unwrap();
    check(&maze, None);
}

#[test]
fn goal_reset_and_pointer_readout_cross_partial_batches() {
    for tail in 1..=8 {
        let end = 16 + tail - 1;
        let mut cells = vec![Cell::Open; end + 1];
        cells[0] = Cell::Start;
        cells[8] = Cell::Goal;
        cells[end] = Cell::Goal;
        let maze = Maze::new(cells, (0..end).map(|i| (i, i + 1))).unwrap();
        check(&maze, Some(&(0..=end).collect::<Vec<_>>()));
    }
}
