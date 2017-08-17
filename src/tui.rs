//! The TUI for the interactive game.
//!
//! # Notes
//! Unfortunately, `termion` uses the somewhat confusing convention that terminal positions are
//! given as `(column, row)`. Therefore, we follow that convention for things that are drawn to the
//! screen (like `Grid`), but anything coming from a `game::Game` or `Sudoku` follows the usual
//! convention of `(row, column)`.

use std::char;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{stdin, stdout, Stdin, Stdout, Write};

use termion::{self, clear, color, cursor, style};
use termion::event::Key;
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
    /// The text to display in the status line.
    status: String,
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
            status: "Welcome to RSudoku!".into(),
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
        self.draw_all(stdout);
        stdout.flush().unwrap();

        for key in stdin.keys() {
            let key = key.unwrap();
            match self.input(key, stdout) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    self.set_status(&format!("Error: {}", e));
                    self.draw_status(stdout);
                    stdout.flush().unwrap();
                }
            }
        }

        Ok(())
    }

    /// Processes keyboard input for normal mode, returning whether the game should exit.
    fn input<W: Write>(&mut self, key: Key, out: &mut W) -> Result<bool> {
        match key {
            Key::Char('q') => return Ok(true),
            // Motion in grid
            Key::Char('h') => self.game.move_by(0, -1),
            Key::Char('j') => self.game.move_by(1, 0),
            Key::Char('k') => self.game.move_by(-1, 0),
            Key::Char('l') => self.game.move_by(0, 1),
            Key::Char('H') => self.game.move_by(0, -3),
            Key::Char('J') => self.game.move_by(3, 0),
            Key::Char('K') => self.game.move_by(-3, 0),
            Key::Char('L') => self.game.move_by(0, 3),
            // Removal
            Key::Char('0') | Key::Char('x') | Key::Char('d') | Key::Delete => self.game.remove(),
            // Insertion
            Key::Char(c @ '1'...'9') => self.game.put(c.to_digit(10).unwrap() as u8),
            // Undo
            Key::Char('u') => {
                if self.game.undo() {
                    self.set_status("Undid last move");
                } else {
                    self.set_status("Nothing to undo");
                }
            }
            // Annotation
            Key::Char('a') => {
                let stdin = stdin();
                match stdin.keys().next().unwrap().unwrap() {
                    Key::Char(c @ '1'...'9') => self.game.annotate(c.to_digit(10).unwrap() as u8),
                    _ => {}
                }
            }
            // Status command
            Key::Char(':') => return self.input_status(out),
            _ => {}
        }

        // We clear the last given hint here; the highlighting will take place at the end of
        // `input_status` and should be cleared on the next action (which is now).
        self.hintpos = None;
        self.draw_all(out);
        out.flush().unwrap();

        Ok(false)
    }

    /// Processes keyboard input for status mode, returning whether the game should exit.
    fn input_status<W: Write>(&mut self, out: &mut W) -> Result<bool> {
        let mut command = String::new();
        let (_, height) = termion::terminal_size().unwrap();
        write!(
            out,
            "{}{}:{}",
            cursor::Goto(1, height),
            clear::CurrentLine,
            cursor::Show
        ).unwrap();
        out.flush().unwrap();

        // By default, we will go back to normal mode at the end of the command
        let stdin = stdin();
        for key in stdin.keys() {
            let key = key.unwrap();
            match key {
                Key::Char('\n') => {
                    let res = self.process_command(&command);
                    write!(out, "{}", cursor::Hide).unwrap();
                    self.draw_all(out);
                    out.flush().unwrap();

                    return res;
                }
                Key::Char(c) => {
                    command.push(c);
                    write!(out, "{}", c).unwrap();
                    out.flush().unwrap();
                }
                Key::Backspace => {
                    if let Some(_) = command.pop() {
                        write!(out, "{0} {0}", cursor::Left(1)).unwrap();
                        out.flush().unwrap();
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    /// Processes the given status command and executes the appropriate function, returning whether
    /// the game should exit.
    fn process_command(&mut self, command: &str) -> Result<bool> {
        match command {
            "q" => return Ok(true),
            "hint" => {
                match self.game.hint()? {
                    Some((row, col)) => {
                        self.hintpos = Some((row, col));
                        self.set_status(&format!("Hint given at position ({}, {})", row, col))
                    }
                    None => self.set_status("No hint could be given"),
                }
            }
            s => self.set_status(&format!("Unknown command '{}'", s)),
        }

        Ok(false)
    }

    /// Draws everything in the TUI.
    fn draw_all<W: Write>(&self, out: &mut W) {
        self.draw_sudoku(out);
        self.draw_status(out);
    }

    /// Draws the status line.
    fn draw_status<W: Write>(&self, out: &mut W) {
        let (_, height) = termion::terminal_size().unwrap();
        write!(
            out,
            "{}{}{}",
            cursor::Goto(1, height),
            clear::CurrentLine,
            self.status
        ).unwrap();
    }

    /// Draws the Sudoku grid (and its contents) to the correct location.
    fn draw_sudoku<W: Write>(&self, out: &mut W) {
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

                draw_in_grid(
                    out,
                    if self.game.board()[i][j] != 0 {
                        char::from_digit(self.game.board()[i][j] as u32, 10).unwrap()
                    } else {
                        '.'
                    },
                    (i as u16, j as u16),
                    startpos,
                );

                write!(out, "{}{}", style::Reset, color::Bg(color::Reset)).unwrap();
            }
        }
    }

    /// Sets the current game status.
    fn set_status(&mut self, status: &str) {
        self.status = status.into();
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

/// Draws the given character at cell position `position` (in a `Grid`) with the given
/// offset.
fn draw_in_grid<W: Write>(out: &mut W, c: char, position: (u16, u16), offset: (u16, u16)) {
    // Compute the position of this cell, relative to `offset`
    let relpos = (
        CELL_WIDTH * position.1 + position.1 / 3 + 1,
        CELL_HEIGHT * position.0 + position.0 / 3 + 1,
    );
    write!(
        out,
        "{}",
        cursor::Goto(offset.0 + relpos.0, offset.1 + relpos.1)
    ).unwrap();
    // We pad with spaces so that background colors can be applied.
    for _ in 0..CELL_WIDTH / 2 {
        write!(out, " ").unwrap();
    }
    write!(out, "{}", c).unwrap();
    for _ in 0..CELL_WIDTH / 2 {
        write!(out, " ").unwrap();
    }
}
