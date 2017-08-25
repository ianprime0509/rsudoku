//! Universal backend for the interactive Sudoku game.

use rand::{self, Rng};

use errors::*;
use sudoku::{Annotations, Sudoku};

/// Represents the state of a game.
pub struct Game {
    /// The board currently in play.
    board: Sudoku,
    /// The original (given) board.
    given: Sudoku,
    /// User annotations.
    annotations: [[Annotations; 9]; 9],
    /// The current position as `(row, column)`.
    position: (usize, usize),
    /// Undo history.
    history: Vec<UndoState>,
    /// Whether the current board has been solved.
    is_solved: bool,
}

/// A snapshot of relevant fields to be restored when using the undo feature.
struct UndoState {
    board: Sudoku,
    annotations: [[Annotations; 9]; 9],
}

impl Game {
    /// Starts a new game with a randomly generated (uniquely solvable) board.
    pub fn new() -> Self {
        let given = Sudoku::generate();

        Game {
            board: given.clone(),
            given,
            annotations: [[Annotations::new(); 9]; 9],
            position: (0, 0),
            history: Vec::new(),
            is_solved: false,
        }
    }

    /// Toggles the given annotation at the current position.
    pub fn annotate(&mut self, n: u8) {
        assert!(1 <= n && n <= 9);
        self.save_state();
        self.annotations[self.position.0][self.position.1].toggle(n);
    }

    /// Returns a reference to the user annotations array.
    pub fn annotations(&self) -> &[[Annotations; 9]; 9] {
        &self.annotations
    }

    /// Returns a reference to the current board.
    pub fn board(&self) -> &Sudoku {
        &self.board
    }

    /// Returns a reference to the originally given board before any user input.
    pub fn given(&self) -> &Sudoku {
        &self.given
    }

    /// Gives a hint for the current board, returning the position of the hint given (or `None` if
    /// the sudoku was already solved). An error will be returned if the current board is in an
    /// invalid state (has no solution).
    pub fn hint(&mut self) -> Result<Option<(usize, usize)>> {
        let s = match self.board.solutions().next() {
            None => return Err(ErrorKind::InvalidSudoku.into()),
            Some(s) => s,
        };

        // All the empty positions in the grid
        let empty = iproduct!(0..9, 0..9)
            .filter(|&(i, j)| self.board[i][j] == 0)
            .collect::<Vec<_>>();
        match rand::thread_rng().choose(&empty) {
            None => Ok(None),
            Some(&(row, col)) => {
                self.save_state();
                self.board.put_at(s[row][col], row, col);
                // Check to see if we've solved the board with this hint
                self.is_solved = self.board.is_solved();
                Ok(Some((row, col)))
            }
        }
    }

    /// Returns whether the current board has been solved.
    pub fn is_solved(&self) -> bool {
        self.is_solved
    }

    /// Moves the current position in the grid by the given amount in each direction.
    /// If the motion in either direction would take the position outside the grid, the position in
    /// that direction will be unchanged.
    pub fn move_by(&mut self, rows: isize, cols: isize) {
        let (row, col) = self.position();
        let (newrow, newcol) = (row as isize + rows, col as isize + cols);
        self.set_position(
            if 0 <= newrow && newrow < 9 {
                newrow as usize
            } else {
                row
            },
            if 0 <= newcol && newcol < 9 {
                newcol as usize

            } else {
                col
            },
        );
    }

    /// Returns the current position in the game grid.
    pub fn position(&self) -> (usize, usize) {
        self.position
    }

    /// Puts the given number at the current position in the game grid.
    ///
    /// If a number is already in the current position as a given, nothing will happen.
    ///
    /// # Panics
    /// Will panic if `n` is not between 1 and 9, inclusive.
    pub fn put(&mut self, n: u8) {
        assert!(1 <= n && n <= 9, "entry number `{}` is invalid", n);

        let (row, col) = self.position;
        if self.given[row][col] == 0 {
            self.save_state();
            self.board.put_at(n, row, col);
            // Check to see if we've solved the board with this move
            self.is_solved = self.board.is_solved();
        }
    }

    /// Removes the number at the current position in the game grid.
    ///
    /// If a number is already in the current position as a given, nothing will happen.
    pub fn remove(&mut self) {
        let (row, col) = self.position;
        if self.given[row][col] == 0 {
            self.save_state();
            self.board.remove_at(row, col);
            // We may have unsolved the board with this move
            self.is_solved = self.board.is_solved();
        }
    }

    /// Sets the current position in the game grid. Note that both the row and the column are
    /// 0-based.
    pub fn set_position(&mut self, row: usize, col: usize) {
        assert!(row < 9 && col < 9, "position ({}, {}) is invalid", row, col);

        self.position = (row, col);
    }

    /// Attempts to solve the board completely, returning `true` if this was actually done.
    pub fn solve(&mut self) -> bool {
        self.board = match self.board.solutions().next() {
            None => return false,
            Some(s) => s,
        };
        self.is_solved = true;
        true
    }

    /// Reverts to the previous game state stored in the history, returning `true` if there was such
    /// a state to revert to and `false` otherwise.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            None => false,
            Some(UndoState { board, annotations }) => {
                self.board = board;
                self.annotations = annotations;
                // Update solved status (shouldn't be necessary, but it doesn't hurt to check)
                self.is_solved = self.board.is_solved();
                true
            }
        }
    }

    /// Saves the current game state to the undo history.
    fn save_state(&mut self) {
        let state = UndoState {
            board: self.board.clone(),
            annotations: self.annotations.clone(),
        };
        self.history.push(state);
    }
}
