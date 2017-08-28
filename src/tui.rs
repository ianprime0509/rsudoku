// Copyright (C) 2017 Ian Johnson
//
// This file is part of RSudoku.
//
// RSudoku is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// RSudoku is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with RSudoku.  If not, see <http://www.gnu.org/licenses/>.
//! The TUI for the interactive game.
//!
//! # Notes
//!
//! Unfortunately, `termion` uses the somewhat confusing convention that terminal positions are
//! given as `(column, row)`. Therefore, we follow that convention for things that are drawn to the
//! screen (like `Grid`), but anything coming from a `game::Game` or `Sudoku` follows the usual
//! convention of `(row, column)`.

use std::char;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{stdin, stdout, Stdout, Write};
use std::ops::Drop;
use std::thread;

use chan::{self, Receiver};
use chan_signal::{self, Signal};
use termion::{self, clear, color, cursor, style};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use errors::*;
use game;
use util;
use Sudoku;

/// The minimum width of the terminal to effectively play the game.
const MIN_WIDTH: u16 = 72;
/// The minimum height of the terminal to effectively play the game.
const MIN_HEIGHT: u16 = 20;

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
const COMMANDS: &[&str] = &["annot", "help", "hint", "new", "noannot", "solve", "q"];

/// A documentation string for the in-game controls.
const HELP: &str = "TUI GAME CONTROLS

CONTROL              DESCRIPTION
hjkl or arrows       movement by cell
HJKL                 movement by 3x3 box
1-9                  fill cell with number
0, d, x, DELETE      clear number in cell
a <number>           toggles annotation for <number> in cell
u                    undo last action
:                    input an ex-style command (see list below)

COMMANDS             DESCRIPTION
:q                   quit the game
:annot               turn on annotations display
:noannot             turn off annotations display
:help                show this help
:hint                give a hint
:new                 start a new game
:solve               solve the current board
";

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
    /// Keyboard input channel.
    keys: Receiver<Key>,
    /// Signal input channel.
    signals: Receiver<Signal>,
}

/// The outline of a grid to be drawn on screen.
///
/// The members of a `Grid` are the width in columns and the height in rows (respectively) of a
/// cell.
struct Grid(u16, u16);

impl<'a> Game<'a> {
    /// Runs the game interactively, using the given `Sudoku` as the initial board.
    pub fn run(s: Sudoku) -> Result<()> {
        // Listen for terminal resize signals.
        // NOTE: this MUST be called before any other threads are spawned, per the `chan_signal`
        // documentation.
        let signals = chan_signal::notify(&[Signal::WINCH]);

        // Set up keyboard input channel
        let (keys_send, keys_recv) = chan::async();
        thread::spawn(move || {
            let stdin = stdin();
            for key in stdin.keys() {
                keys_send.send(key.unwrap());
            }
        });

        let mut stdout = stdout().into_raw_mode().unwrap();
        // As part of the display setup, we hide the cursor; when the `Game` is dropped, the cursor
        // will be shown again. This logic is moved to the `Drop` implementation so that it is
        // guaranteed to happen even if we exit on an error somehow.
        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            cursor::Hide,
        ).unwrap();
        stdout.flush().unwrap();

        let mut game = Game {
            game: game::Game::from_sudoku(s),
            hintpos: None,
            status: "Welcome to RSudoku! Type `:help<RET>` for help.".into(),
            show_annotations: false,
            stdout: &mut stdout,
            keys: keys_recv,
            signals,
        };
        game.main()?;

        Ok(())
    }

    /// Runs the game.
    fn main(&mut self) -> Result<()> {
        // Check to see if terminal size is too small; if so, nothing will be drawn and the user
        // may be very confused, so it's best to just exit with an error if this is the case
        // initially.
        let (width, height) = termion::terminal_size().unwrap();
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            bail!(
                "terminal is too small to play the game; must be at least {} rows by {} columns \
                 (current terminal has {} rows and {} columns)",
                MIN_HEIGHT,
                MIN_WIDTH,
                height,
                width
            );
        }
        // Draw initial game view, so that it displays before any input is entered
        self.draw_all();
        self.stdout.flush().unwrap();

        loop {
            // I have no idea why the `chan_select` macro doesn't accept anything with `self` in it,
            // but this works just as well I guess...
            let keys = self.keys.clone();
            let signals = self.signals.clone();
            chan_select! {
                keys.recv() -> key => {
                    match self.input_key(key.unwrap()) {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            self.set_status(&format!("Error: {}", e));
                            self.draw_status();
                            self.stdout.flush().unwrap();
                        }
                    }
                },
                signals.recv() -> signal => {
                    match signal.unwrap() {
                        Signal::WINCH => {
                            write!(self.stdout, "{}", clear::All).unwrap();
                            self.draw_all();
                            self.stdout.flush().unwrap();
                        }
                        _ => {}
                    }
                },
            }
        }

        Ok(())
    }

    /// Processes keyboard input for normal mode, returning whether the game should exit.
    fn input_key(&mut self, key: Key) -> Result<bool> {
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
                Key::Char('h') | Key::Left => self.game.move_by(0, -1),
                Key::Char('j') | Key::Down => self.game.move_by(1, 0),
                Key::Char('k') | Key::Up => self.game.move_by(-1, 0),
                Key::Char('l') | Key::Right => self.game.move_by(0, 1),
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
                    match self.keys.recv().unwrap() {
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

        for key in &self.keys {
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
            "help" => self.show_help()?,
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
        self.draw_status()
    }

    /// Draws the annotations window and its contents. This should only be used if annotations are
    /// enabled.
    fn draw_annotations(&mut self) {
        assert!(self.show_annotations);
        let (width, height) = termion::terminal_size().unwrap();
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            return;
        }
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
        if height < MIN_HEIGHT {
            return;
        }
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
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            return;
        }
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

    /// Shows the game help.
    fn show_help(&mut self) -> Result<()> {
        let (_, height) = termion::terminal_size().unwrap();
        write!(
            self.stdout,
            "{}{}{}{}{}{}",
            cursor::Hide,
            clear::All,
            cursor::Goto(1, 1),
            // Since we're in raw mode, '\n' only means "move one row down"
            HELP.replace('\n', "\r\n"),
            cursor::Goto(1, height),
            "(press any key to close help)"
        ).unwrap();
        self.stdout.flush().unwrap();

        // Wait for a key
        self.keys.recv().unwrap();

        write!(self.stdout, "{}{}", clear::All, cursor::Show).unwrap();
        self.draw_all();
        self.stdout.flush().unwrap();
        Ok(())
    }
}

impl<'a> Drop for Game<'a> {
    fn drop(&mut self) {
        // Clean up the terminal display
        write!(
            self.stdout,
            "{}{}{}",
            cursor::Show,
            cursor::Goto(1, 1),
            clear::All
        ).unwrap();
        self.stdout.flush().unwrap();
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
