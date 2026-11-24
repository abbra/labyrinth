//! Grid mazes parsed from ASCII art.

use std::fmt;
use std::str::FromStr;

/// A cell coordinate, `row` counted from the top and `col` from the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    /// Zero-based row.
    pub row: usize,
    /// Zero-based column.
    pub col: usize,
}

/// One maze cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cell {
    /// Impassable.
    Wall,
    /// Passable.
    Open,
    /// Passable; where the path begins.
    Start,
    /// Passable; where the path ends.
    Goal,
}

/// A rectangular maze with exactly one start and one goal.
///
/// Text form: `#` wall, `.` or space open, `S` start, `G` goal; one row per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maze {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    start: Pos,
    goal: Pos,
}

impl Maze {
    /// Number of columns.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Position of the `S` cell.
    pub fn start(&self) -> Pos {
        self.start
    }

    /// Position of the `G` cell.
    pub fn goal(&self) -> Pos {
        self.goal
    }

    /// The cell at `pos`, or `None` outside the grid.
    pub fn cell(&self, pos: Pos) -> Option<Cell> {
        (pos.row < self.height && pos.col < self.width)
            .then(|| self.cells[pos.row * self.width + pos.col])
    }
}

/// Why a maze string was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseMazeError {
    /// No rows.
    Empty,
    /// A row's length differs from the first row's.
    Ragged {
        /// Zero-based row index.
        row: usize,
    },
    /// A character other than `#`, `.`, space, `S`, `G`.
    BadChar {
        /// The offending character.
        ch: char,
        /// Where it was found.
        pos: Pos,
    },
    /// Zero or several `S` cells.
    Start,
    /// Zero or several `G` cells.
    Goal,
}

impl fmt::Display for ParseMazeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("maze has no rows"),
            Self::Ragged { row } => write!(f, "row {row} has a different width than row 0"),
            Self::BadChar { ch, pos } => {
                write!(
                    f,
                    "unexpected {ch:?} at row {}, column {}",
                    pos.row, pos.col
                )
            }
            Self::Start => f.write_str("maze needs exactly one 'S'"),
            Self::Goal => f.write_str("maze needs exactly one 'G'"),
        }
    }
}

impl std::error::Error for ParseMazeError {}

impl FromStr for Maze {
    type Err = ParseMazeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rows: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
        let width = rows.first().ok_or(ParseMazeError::Empty)?.chars().count();
        let (mut cells, mut starts, mut goals) = (Vec::new(), Vec::new(), Vec::new());
        for (row, line) in rows.iter().enumerate() {
            if line.chars().count() != width {
                return Err(ParseMazeError::Ragged { row });
            }
            for (col, ch) in line.chars().enumerate() {
                let pos = Pos { row, col };
                cells.push(match ch {
                    '#' => Cell::Wall,
                    '.' | ' ' => Cell::Open,
                    'S' => {
                        starts.push(pos);
                        Cell::Start
                    }
                    'G' => {
                        goals.push(pos);
                        Cell::Goal
                    }
                    ch => return Err(ParseMazeError::BadChar { ch, pos }),
                });
            }
        }
        let [start] = starts[..] else {
            return Err(ParseMazeError::Start);
        };
        let [goal] = goals[..] else {
            return Err(ParseMazeError::Goal);
        };
        Ok(Self {
            width,
            height: rows.len(),
            cells,
            start,
            goal,
        })
    }
}
