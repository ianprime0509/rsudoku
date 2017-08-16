//! The TUI for the interactive game.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{stdout, Write};

use termion::{clear, cursor};

use errors::*;
use game;

/// Contains the state of the TUI game.
pub struct Game {
    /// The underlying game state.
    game: game::Game,
    /// The position of the last hint given (for highlighting).
    hintpos: Option<(usize, usize)>,
}

/// The outline of a grid to be drawn on screen.
///
/// The members of a `Grid` are the height in rows and the width in columns (respectively) of a
/// cell.
struct Grid(u16, u16);

impl Game {
    /// Runs the game interactively.
    pub fn run() -> Result<()> {
        let game = Game {
            game: game::Game::new(),
            hintpos: None,
        };

        print!("{}{}{}", clear::All, cursor::Goto(1, 1), Grid(1, 3));
        stdout().flush().unwrap();

        Ok(())
    }
}

impl Grid {
    pub fn height(&self) -> u16 {
        4 + 9 * self.0
    }

    pub fn width(&self) -> u16 {
        4 + 9 * self.1
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        // Top row
        write!(f, "╔")?;
        for _ in 0..2 {
            for _ in 0..3 * self.1 {
                write!(f, "═")?;
            }
            write!(f, "╤")?;
        }
        for _ in 0..3 * self.1 {
            write!(f, "═")?;
        }
        write!(f, "╗")?;
        write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;

        // Middle section
        for _ in 0..2 {
            for _ in 0..3 * self.0 {
                write!(f, "║")?;
                for _ in 0..2 {
                    write!(f, "{}", cursor::Right(3 * self.1))?;
                    write!(f, "│")?;
                }
                write!(f, "{}", cursor::Right(3 * self.1))?;
                write!(f, "║")?;
                write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
            }
            // Divider row
            write!(f, "╟")?;
            for _ in 0..2 {
                for _ in 0..3 * self.1 {
                    write!(f, "─")?;
                }
                write!(f, "┼")?;
            }
            for _ in 0..3 * self.1 {
                write!(f, "─")?;
            }
            write!(f, "╢")?;
            write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
        }
        for _ in 0..3 * self.0 {
            write!(f, "║")?;
            for _ in 0..2 {
                write!(f, "{}", cursor::Right(3 * self.1))?;
                write!(f, "│")?;
            }
            write!(f, "{}", cursor::Right(3 * self.1))?;
            write!(f, "║")?;
            write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
        }

        // Bottom row
        write!(f, "╚")?;
        for _ in 0..2 {
            for _ in 0..3 * self.1 {
                write!(f, "═")?;
            }
            write!(f, "╧")?;
        }
        for _ in 0..3 * self.1 {
            write!(f, "═")?;
        }
        write!(f, "╝")?;

        Ok(())
    }
}
