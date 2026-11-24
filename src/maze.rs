//! Mazes as graphs of cells, so any tiling works: squares, hexagons, triangles,
//! aperiodic Penrose rhombs. Square grids can also be parsed from ASCII art.

use std::fmt;
use std::str::FromStr;

/// Maximum neighbours supported by the solver's fixed rules.
pub const MAX_DEGREE: usize = 8;

/// One maze cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cell {
    /// Impassable.
    Wall,
    /// Passable.
    Open,
    /// Passable; where the path begins.
    Start,
    /// Passable; a cell the path must visit.
    Goal,
}

/// Cells numbered `0..n` plus which ones share a side, with one start and one or more goals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maze {
    cells: Vec<Cell>,
    /// Neighbours of cell `i` are `targets[offsets[i]..offsets[i + 1]]`.
    offsets: Vec<usize>,
    targets: Vec<usize>,
    start: usize,
    goals: Vec<usize>,
}

impl Maze {
    /// Builds a maze from its cells and the undirected `edges` between them.
    ///
    /// Each cell's neighbours keep the order its edges appear in.
    ///
    /// # Errors
    ///
    /// Rejects edges to unknown cells, self-loops, repeated edges, cells with more than
    /// [`MAX_DEGREE`] neighbours, anything but exactly one start, and mazes with no goal.
    pub fn new(
        cells: Vec<Cell>,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self, MazeError> {
        let n = cells.len();
        let edges: Vec<_> = edges.into_iter().collect();
        let mut degree = vec![0; n];
        for &(a, b) in &edges {
            if a >= n || b >= n {
                return Err(MazeError::UnknownCell { edge: (a, b) });
            }
            if a == b {
                return Err(MazeError::SelfLoop { cell: a });
            }
            for c in [a, b] {
                degree[c] += 1;
                if degree[c] > MAX_DEGREE {
                    return Err(MazeError::TooManyNeighbours { cell: c });
                }
            }
        }
        let mut offsets = Vec::with_capacity(n + 1);
        offsets.push(0);
        for d in &degree {
            offsets.push(offsets.last().unwrap() + d);
        }
        let mut fill = offsets[..n].to_vec();
        let mut targets = vec![0; 2 * edges.len()];
        for &(a, b) in &edges {
            for (from, to) in [(a, b), (b, a)] {
                if targets[offsets[from]..fill[from]].contains(&to) {
                    return Err(MazeError::DuplicateEdge { edge: (a, b) });
                }
                targets[fill[from]] = to;
                fill[from] += 1;
            }
        }
        let find = |kind| {
            let mut found = (0..n).filter(|&i| cells[i] == kind);
            match (found.next(), found.next()) {
                (Some(i), None) => Some(i),
                _ => None,
            }
        };
        let start = find(Cell::Start).ok_or(MazeError::Start)?;
        let goals: Vec<_> = (0..n).filter(|&i| cells[i] == Cell::Goal).collect();
        if goals.is_empty() {
            return Err(MazeError::Goal);
        }
        Ok(Self {
            cells,
            offsets,
            targets,
            start,
            goals,
        })
    }

    /// Every cell, indexed by cell id.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Cells sharing a side with `cell`.
    ///
    /// # Panics
    ///
    /// Panics if `cell` is not below `cells().len()`.
    pub fn neighbours(&self, cell: usize) -> &[usize] {
        &self.targets[self.offsets[cell]..self.offsets[cell + 1]]
    }

    /// Id of the start cell.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Ids of all goal cells, in cell order.
    pub fn goals(&self) -> &[usize] {
        &self.goals
    }

    /// Id of the first goal cell.
    ///
    /// For mazes with several goals, use [`Maze::goals`].
    pub fn goal(&self) -> usize {
        self.goals[0]
    }
}

/// Why a maze was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MazeError {
    /// Text with no rows.
    Empty,
    /// A text row's length differs from the first row's.
    Ragged {
        /// Zero-based row index.
        row: usize,
    },
    /// A character other than `#`, `.`, space, `S`, `G`.
    BadChar {
        /// The offending character.
        ch: char,
        /// Zero-based row.
        row: usize,
        /// Zero-based column.
        col: usize,
    },
    /// An edge names a cell id that does not exist.
    UnknownCell {
        /// The offending edge.
        edge: (usize, usize),
    },
    /// An edge joins a cell to itself.
    SelfLoop {
        /// The cell.
        cell: usize,
    },
    /// The same pair of cells is joined twice.
    DuplicateEdge {
        /// The second occurrence.
        edge: (usize, usize),
    },
    /// A cell has more than [`MAX_DEGREE`] neighbours.
    TooManyNeighbours {
        /// The cell.
        cell: usize,
    },
    /// Zero start cells, or more than one.
    Start,
    /// Zero goal cells.
    Goal,
}

impl fmt::Display for MazeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("maze has no rows"),
            Self::Ragged { row } => write!(f, "row {row} has a different width than row 0"),
            Self::BadChar { ch, row, col } => {
                write!(f, "unexpected {ch:?} at row {row}, column {col}")
            }
            Self::UnknownCell { edge } => write!(f, "edge {edge:?} names an unknown cell"),
            Self::SelfLoop { cell } => write!(f, "cell {cell} is its own neighbour"),
            Self::DuplicateEdge { edge } => write!(f, "edge {edge:?} appears twice"),
            Self::TooManyNeighbours { cell } => {
                write!(f, "cell {cell} has more than {MAX_DEGREE} neighbours")
            }
            Self::Start => f.write_str("maze needs exactly one start"),
            Self::Goal => f.write_str("maze needs at least one goal"),
        }
    }
}

impl std::error::Error for MazeError {}

/// Square grid from text: `#` wall, `.` or space open, `S` start, `G` goal; one row per
/// line. Cell ids are row-major: row `r`, column `c` is `r * width + c`.
impl FromStr for Maze {
    type Err = MazeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rows: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
        let width = rows.first().ok_or(MazeError::Empty)?.chars().count();
        let mut cells = Vec::with_capacity(width * rows.len());
        let mut edges = Vec::with_capacity(2 * width * rows.len());
        for (row, line) in rows.iter().enumerate() {
            if line.chars().count() != width {
                return Err(MazeError::Ragged { row });
            }
            for (col, ch) in line.chars().enumerate() {
                let i = cells.len();
                cells.push(match ch {
                    '#' => Cell::Wall,
                    '.' | ' ' => Cell::Open,
                    'S' => Cell::Start,
                    'G' => Cell::Goal,
                    ch => return Err(MazeError::BadChar { ch, row, col }),
                });
                if col > 0 {
                    edges.push((i - 1, i));
                }
                if row > 0 {
                    edges.push((i - width, i));
                }
            }
        }
        Self::new(cells, edges)
    }
}
