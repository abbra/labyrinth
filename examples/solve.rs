//! Reads a maze from stdin and prints it with the model's path marked `*`.

use std::io::{self, Read};

use labyrinth::{Maze, Model};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    let maze: Maze = text.parse()?;
    let model = Model::new();
    let Some(path) = model.solve(&maze) else {
        println!("no path");
        return Ok(());
    };
    let mut rows: Vec<Vec<char>> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().collect())
        .collect();
    for p in &path[1..path.len() - 1] {
        rows[p.row][p.col] = '*';
    }
    for row in rows {
        println!("{}", String::from_iter(row));
    }
    println!(
        "{} steps, {} weights",
        path.len() - 1,
        model.parameter_count()
    );
    Ok(())
}
