//! The TUI for the interactive game.
//!
//! # Notes
//! Unfortunately, `termion` uses the somewhat confusing convention that terminal positions are
//! given as `(column, row)`. Therefore, we follow that convention for things that are drawn to the
//! screen (like `Grid`), but anything coming from a `game::Game` or `Sudoku` follows the usual
//! convention of `(row, column)`.

use std::char;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{stdin, stdout, Stdout, Write};

use termion::{self, clear, color, cursor, style};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use errors::*;
use game;
use util;

/// The width of a single cell in the sudoku grid; must be odd.
const CELL_WIDTH: u16 = 3;
/// The height of a single cell in the sudoku grid; must be odd.
/// Right now, any value other than 1 won't be handled quite correctly.
const CELL_HEIGHT: u16 = 1;

/// The background color to use for highlighting the most recent hint.
const COLOR_HINT: color::Yellow = color::Yellow;
/// The background color to use for highlighting the selected cell.
const COLOR_SELECTION: color::Blue = color::Blue;
/// The background color to use for indicating that the board has been solved.
const COLOR_SOLVED: color::Green = color::Green;

/// All possible status commands.
const COMMANDS: &[&str] = &["annot", "hint", "new", "noannot", "solve", "q"];

/// Contains the state of the TUI game.
pub struct Game<'a> {
    /// The underlying game state.
    game: game::Game,
    /// The position of the last hint given (for highlighting).
    hintpos: Option<(usize, usize)>,
    /// The text to display in the status line.
    status: String,
    /// Whether to show the annotations window.
    show_annotations: bool,
    /// The underlying terminal output.
    stdout: &'a mut RawTerminal<Stdout>,
}

/// The outline of a grid to be drawn on screen.
///
/// The members of a `Grid` are the width in columns and the height in rows (respectively) of a
/// cell.
struct Grid(u16, u16);

impl<'a> Game<'a> {
    /// Runs the game interactively.
    pub fn run() -> Result<()> {
        let mut stdout = stdout().into_raw_mode().unwrap();
        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            cursor::Hide,
        ).unwrap();
        stdout.flush().unwrap();

        {
            let mut game = Game {
                game: game::Game::new(),
                hintpos: None,
                status: "Welcome to RSudoku!".into(),
                show_annotations: false,
                stdout: &mut stdout,
            };
            game.main()?;
        }

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
    fn main(&mut self) -> Result<()> {
        self.draw_all();
        self.stdout.flush().unwrap();

        let stdin = stdin();
        for key in stdin.keys() {
            let key = key.unwrap();
            match self.input(key) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    self.set_status(&format!("Error: {}", e));
                    self.draw_status();
                    self.stdout.flush().unwrap();
                }
            }
        }

        Ok(())
    }

    /// Processes keyboard input for normal mode, returning whether the game should exit.
    fn input(&mut self, key: Key) -> Result<bool> {
        // We clear the status on each new iteration of the input loop so that the message doesn't
        // stick around forever (it will be visible until the user does something).
        self.set_status("");

        // We handle this case separately so that we can run status commands even after the game has
        // been solved; other (normal) commands do not work in this state.
        if key == Key::Char(':') {
            return self.input_status();
        }
        if !self.game.is_solved() {
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
                Key::Char('0') | Key::Char('x') | Key::Char('d') | Key::Delete => {
                    self.game.remove()
                }
                // Insertion
                Key::Char(c @ '1'...'9') => {
                    self.game.put(c.to_digit(10).unwrap() as u8);
                    self.check_solved();
                }
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
                        Key::Char(c @ '1'...'9') => {
                            self.game.annotate(c.to_digit(10).unwrap() as u8)
                        }
                        _ => self.set_status("Must enter a number (1-9) to annotate"),
                    }
                }
                _ => {}
            }
        }

        // We clear the last given hint here; the highlighting will take place at the end of
        // `input_status` and should be cleared on the next action (which is now).
        self.hintpos = None;
        self.draw_all();
        self.stdout.flush().unwrap();

        Ok(false)
    }

    /// Processes keyboard input for status mode, returning whether the game should exit.
    fn input_status(&mut self) -> Result<bool> {
        let mut command = String::new();
        let (_, height) = termion::terminal_size().unwrap();
        write!(
            self.stdout,
            "{}{}:{}",
            cursor::Goto(1, height),
            clear::CurrentLine,
            cursor::Show
        ).unwrap();
        self.stdout.flush().unwrap();

        let stdin = stdin();
        for key in stdin.keys() {
            let key = key.unwrap();
            match key {
                Key::Char('\n') => {
                    let res = self.process_command(&command);
                    write!(self.stdout, "{}{}", clear::CurrentLine, cursor::Hide).unwrap();
                    self.draw_all();
                    self.stdout.flush().unwrap();

                    return res;
                }
                Key::Char('\t') => {
                    let completion = util::complete(COMMANDS, &command);
                    for c in completion.chars() {
                        command.push(c);
                        write!(self.stdout, "{}", c).unwrap();
                    }
                    self.stdout.flush().unwrap();
                }
                Key::Char(c) => {
                    command.push(c);
                    write!(self.stdout, "{}", c).unwrap();
                    self.stdout.flush().unwrap();
                }
                Key::Backspace => {
                    write!(self.stdout, "{0} {0}", cursor::Left(1)).unwrap();
                    self.stdout.flush().unwrap();
                    // Cancel command entry if the user tries to backspace over the leading ':'
                    if command.pop() == None {
                        write!(self.stdout, "{}{}", clear::CurrentLine, cursor::Hide).unwrap();
                        self.stdout.flush().unwrap();
                        return Ok(false);
                    }
                }
                Key::Esc => {
                    write!(self.stdout, "{}{}", clear::CurrentLine, cursor::Hide).unwrap();
                    self.stdout.flush().unwrap();
                    return Ok(false);
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
            "" => return Ok(false),
            "q" => return Ok(true),
            "annot" => {
                self.show_annotations = true;
                write!(self.stdout, "{}", clear::All).unwrap();
                self.set_status("Turned on annotations display");
            }
            "hint" => {
                match self.game.hint()? {
                    Some((row, col)) => {
                        self.hintpos = Some((row, col));
                        self.set_status(&format!("Hint given at position ({}, {})", row, col))
                    }
                    None => self.set_status("Current board is already solved"),
                }
                self.check_solved();
            }
            "new" => {
                self.game = game::Game::new();
                self.hintpos = None;
                self.set_status("Started new game");
            }
            "noannot" => {
                self.show_annotations = false;
                write!(self.stdout, "{}", clear::All).unwrap();
                self.set_status("Turned off annotations display");
            }
            "solve" => {
                if !self.game.solve() {
                    self.set_status("Current board has no solution in this state");
                }
                self.check_solved();
            }
            s => self.set_status(&format!("Unknown command '{}'", s)),
        }

        Ok(false)
    }

    /// Draws everything in the TUI.
    fn draw_all(&mut self) {
        self.draw_sudoku();
        if self.show_annotations {
            self.draw_annotations();
        }
        self.draw_status();
    }

    /// Draws the annotations window and its contents. This should only be used if annotations are
    /// enabled.
    fn draw_annotations(&mut self) {
        assert!(
            self.show_annotations,
            "attempted to draw the annotations window with annotations turned off"
        );
        let (width, height) = termion::terminal_size().unwrap();
        let grid = Grid(CELL_WIDTH, CELL_HEIGHT);
        let startpos = (width / 2, height / 2 - grid.height() / 2);
        // The grid position of the top left corner of the 3x3 block we are currently in
        let boxpos = (
            self.game.position().0 / 3 * 3,
            self.game.position().1 / 3 * 3,
        );

        // Draw grid
        write!(self.stdout, "{}", cursor::Goto(startpos.0, startpos.1)).unwrap();
        if self.game.is_solved() {
            write!(
                self.stdout,
                "{}{}{}",
                color::Bg(COLOR_SOLVED),
                grid,
                color::Bg(color::Reset)
            ).unwrap();
        } else {
            write!(self.stdout, "{}", grid).unwrap();
        }
        // Draw contents
        for i in 0..9 {
            for j in 0..9 {
                // The grid position of the cell whose annotations we should draw
                let cellpos = (boxpos.0 + i / 3, boxpos.1 + j / 3);
                // The number of the annotation that we should draw (1-9)
                let n = (3 * (i % 3) + j % 3 + 1) as u8;

                // Highlight selected cell
                if cellpos == self.game.position() {
                    write!(self.stdout, "{}", color::Bg(COLOR_SELECTION)).unwrap();
                }
                // Highlight hinted cell
                if Some(cellpos) == self.hintpos {
                    write!(self.stdout, "{}", color::Bg(COLOR_HINT)).unwrap();
                }
                // Change background color if solved
                if self.game.is_solved() {
                    write!(self.stdout, "{}", color::Bg(COLOR_SOLVED)).unwrap();
                }

                if self.game.annotations()[cellpos.0][cellpos.1][n] {
                    self.draw_in_grid(
                        char::from_digit(n as u32, 10).unwrap(),
                        (i as u16, j as u16),
                        startpos,
                    );
                } else {
                    self.draw_in_grid('.', (i as u16, j as u16), startpos);
                }

                write!(self.stdout, "{}", color::Bg(color::Reset)).unwrap();
            }
        }
    }

    /// Draws the status line.
    fn draw_status(&mut self) {
        let (_, height) = termion::terminal_size().unwrap();
        write!(
            self.stdout,
            "{}{}{}",
            cursor::Goto(1, height),
            clear::CurrentLine,
            self.status
        ).unwrap();
    }

    /// Draws the Sudoku grid (and its contents) to the correct location.
    fn draw_sudoku(&mut self) {
        let (width, height) = termion::terminal_size().unwrap();
        let grid = Grid(CELL_WIDTH, CELL_HEIGHT);
        let startpos = if self.show_annotations {
            (width / 2 - grid.width(), height / 2 - grid.height() / 2)
        } else {
            (width / 2 - grid.width() / 2, height / 2 - grid.height() / 2)
        };

        // Draw grid
        write!(self.stdout, "{}", cursor::Goto(startpos.0, startpos.1)).unwrap();
        if self.game.is_solved() {
            write!(
                self.stdout,
                "{}{}{}",
                color::Bg(COLOR_SOLVED),
                grid,
                color::Bg(color::Reset)
            ).unwrap();
        } else {
            write!(self.stdout, "{}", grid).unwrap();
        }
        // Draw contents
        for i in 0..9 {
            for j in 0..9 {
                // Bold given entries
                if self.game.given()[i][j] != 0 {
                    write!(self.stdout, "{}", style::Bold).unwrap();
                }
                // Highlight selection
                if (i, j) == self.game.position() {
                    write!(self.stdout, "{}", color::Bg(COLOR_SELECTION)).unwrap();
                }
                // Highlight most recent hint
                if Some((i, j)) == self.hintpos {
                    write!(self.stdout, "{}", color::Bg(COLOR_HINT)).unwrap();
                }
                // Change background color if solved
                if self.game.is_solved() {
                    write!(self.stdout, "{}", color::Bg(COLOR_SOLVED)).unwrap();
                }

                if self.game.board()[i][j] != 0 {
                    let c = char::from_digit(self.game.board()[i][j] as u32, 10).unwrap();
                    self.draw_in_grid(c, (i as u16, j as u16), startpos);
                } else {
                    self.draw_in_grid('.', (i as u16, j as u16), startpos);
                }

                write!(self.stdout, "{}{}", style::Reset, color::Bg(color::Reset)).unwrap();
            }
        }
    }

    /// Draws the given character at cell position `position` (relative to a `Grid`) with the given
    /// offset.
    fn draw_in_grid(&mut self, c: char, position: (u16, u16), offset: (u16, u16)) {
        // Compute the position of this cell, relative to `offset`
        let relpos = (
            CELL_WIDTH * position.1 + position.1 / 3 + 1,
            CELL_HEIGHT * position.0 + position.0 / 3 + 1,
        );
        write!(
            self.stdout,
            "{}",
            cursor::Goto(offset.0 + relpos.0, offset.1 + relpos.1)
        ).unwrap();
        // We pad with spaces so that background colors can be applied.
        for _ in 0..CELL_WIDTH / 2 {
            write!(self.stdout, " ").unwrap();
        }
        write!(self.stdout, "{}", c).unwrap();
        for _ in 0..CELL_WIDTH / 2 {
            write!(self.stdout, " ").unwrap();
        }
    }

    /// Checks if the current board has been solved and updates the status accordingly if so.
    fn check_solved(&mut self) {
        if self.game.is_solved() {
            self.set_status("Congratulations, you win!");
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
