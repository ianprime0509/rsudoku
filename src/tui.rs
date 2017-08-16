//! The TUI for the interactive game.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{stdin, stdout, Stdin, Stdout, Write};

use termion::{self, clear, color, cursor, style};
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use errors::*;
use game;

/// The width of a single cell in the sudoku grid; must be odd.
const CELL_WIDTH: u16 = 3;
/// The height of a single cell in the sudoku grid; must be odd.
/// Right now, any value other than 1 won't be handled quite correctly.
const CELL_HEIGHT: u16 = 1;

/// The background color to use for highlighting the selected cell.
const COLOR_SELECTION: color::Blue = color::Blue;
/// The background color to use for highlighting the most recent hint.
const COLOR_HINT: color::Yellow = color::Yellow;

/// Contains the state of the TUI game.
pub struct Game {
    /// The underlying game state.
    game: game::Game,
    /// The position of the last hint given (for highlighting).
    hintpos: Option<(usize, usize)>,
    /// The current mode of the game, analogous to vi's modes.
    mode: Mode,
}

/// The possible game modes.
enum Mode {
    /// Normal navigation, input, etc.
    Normal,
    /// Inputting a status command.
    Status,
    /// Signals that the game should exit.
    Exit,
}

/// The outline of a grid to be drawn on screen.
///
/// The members of a `Grid` are the width in columns and the height in rows (respectively) of a
/// cell.
struct Grid(u16, u16);

impl Game {
    /// Runs the game interactively.
    pub fn run() -> Result<()> {
        let mut game = Game {
            game: game::Game::new(),
            hintpos: None,
            mode: Mode::Normal,
        };
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode().unwrap();

        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            cursor::Hide,
        ).unwrap();
        stdout.flush().unwrap();

        game.main(stdin, &mut stdout)?;

        write!(
            stdout,
            "{}{}{}",
            cursor::Show,
            cursor::Goto(1, 1),
            clear::All
        ).unwrap();
        stdout.flush().unwrap();

        Ok(())
    }

    /// The main game loop.
    fn main(&mut self, stdin: Stdin, mut stdout: &mut RawTerminal<Stdout>) -> Result<()> {
        self.draw_sudoku(&mut stdout)?;
        stdout.flush().unwrap();

        for key in stdin.keys() {
            let key = key.unwrap();
            match self.mode {
                Mode::Normal => self.input_normal(key, stdout)?,
                Mode::Status => {}
                Mode::Exit => break,
            }
        }

        Ok(())
    }

    /// Processes keyboard input for normal mode.
    fn input_normal<W: Write>(&mut self, key: Key, out: &mut W) -> Result<()> {
        match key {
            Key::Char('q') => {
                self.mode = Mode::Exit;
            }
            _ => {}
        }
        Ok(())
    }

    /// Draws the Sudoku grid (and its contents) to the correct location.
    fn draw_sudoku<W: Write>(&self, out: &mut W) -> Result<()> {
        let (width, height) = termion::terminal_size().unwrap();
        let grid = Grid(CELL_WIDTH, CELL_HEIGHT);
        let startpos = (width / 2 - grid.width() / 2, height / 2 - grid.height() / 2);

        // Draw grid
        write!(out, "{}{}", cursor::Goto(startpos.0, startpos.1), grid).unwrap();
        // Draw contents
        for i in 0..9 {
            for j in 0..9 {
                // Bold given entries
                if self.game.given()[i][j] != 0 {
                    write!(out, "{}", style::Bold).unwrap();
                }
                // Highlight selection
                if (i, j) == self.game.position() {
                    write!(out, "{}", color::Bg(COLOR_SELECTION)).unwrap();
                }
                // Highlight most recent hint
                if Some((i, j)) == self.hintpos {
                    write!(out, "{}", color::Bg(COLOR_HINT)).unwrap();
                }

                // Compute the position of this cell, relative to startpos
                let relpos = (
                    CELL_WIDTH * j as u16 + j as u16 / 3 + 1,
                    CELL_HEIGHT * i as u16 + i as u16 / 3 + 1,
                );
                write!(
                    out,
                    "{}",
                    cursor::Goto(startpos.0 + relpos.0, startpos.1 + relpos.1),
                ).unwrap();
                // We pad with spaces so that background colors can be applied.
                for _ in 0..CELL_WIDTH / 2 {
                    write!(out, " ").unwrap();
                }
                if self.game.board()[i][j] != 0 {
                    write!(out, "{}", self.game.board()[i][j]).unwrap();
                } else {
                    write!(out, ".").unwrap();
                }
                for _ in 0..CELL_WIDTH / 2 {
                    write!(out, " ").unwrap();
                }

                write!(out, "{}{}", style::Reset, color::Bg(color::Reset)).unwrap();
            }
        }

        Ok(())
    }
}

impl Grid {
    pub fn height(&self) -> u16 {
        4 + 9 * self.1
    }

    pub fn width(&self) -> u16 {
        4 + 9 * self.0
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        // Top row
        write!(f, "╔")?;
        for _ in 0..2 {
            for _ in 0..3 * self.0 {
                write!(f, "═")?;
            }
            write!(f, "╤")?;
        }
        for _ in 0..3 * self.0 {
            write!(f, "═")?;
        }
        write!(f, "╗")?;
        write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;

        // Middle section
        for _ in 0..2 {
            for _ in 0..3 * self.1 {
                write!(f, "║")?;
                for _ in 0..2 {
                    write!(f, "{}", cursor::Right(3 * self.0))?;
                    write!(f, "│")?;
                }
                write!(f, "{}", cursor::Right(3 * self.0))?;
                write!(f, "║")?;
                write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
            }
            // Divider row
            write!(f, "╟")?;
            for _ in 0..2 {
                for _ in 0..3 * self.0 {
                    write!(f, "─")?;
                }
                write!(f, "┼")?;
            }
            for _ in 0..3 * self.0 {
                write!(f, "─")?;
            }
            write!(f, "╢")?;
            write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
        }
        for _ in 0..3 * self.1 {
            write!(f, "║")?;
            for _ in 0..2 {
                write!(f, "{}", cursor::Right(3 * self.0))?;
                write!(f, "│")?;
            }
            write!(f, "{}", cursor::Right(3 * self.0))?;
            write!(f, "║")?;
            write!(f, "{}{}", cursor::Down(1), cursor::Left(self.width()))?;
        }

        // Bottom row
        write!(f, "╚")?;
        for _ in 0..2 {
            for _ in 0..3 * self.0 {
                write!(f, "═")?;
            }
            write!(f, "╧")?;
        }
        for _ in 0..3 * self.0 {
            write!(f, "═")?;
        }
        write!(f, "╝")?;

        Ok(())
    }
}
