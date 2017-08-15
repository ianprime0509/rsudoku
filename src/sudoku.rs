use std::default::Default;
use std::fmt::{self, Debug, Display, Formatter, Result as FmtResult};
use std::iter::Iterator;
use std::ops::{Index, IndexMut};
use std::str::FromStr;

use errors::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Annotations([bool; 9]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sudoku {
    grid: [[u8; 9]; 9],
    hints: [[Annotations; 9]; 9],
    annotations: [[Annotations; 9]; 9],
}

/// An iterator over all solutions of a particular sudoku.
pub struct Solutions {
    /// The stack currently in use for backtracking.
    stack: Vec<Sudoku>,
}

impl Annotations {
    /// Creates a set of empty annotations.
    pub fn new() -> Annotations {
        Annotations([false; 9])
    }

    /// Creates the annotations corresponding to the given `bool` array.
    pub fn from_array(arr: [bool; 9]) -> Annotations {
        Annotations(arr)
    }

    /// Clears all annotations.
    pub fn clear(&mut self) {
        for b in self.0.iter_mut() {
            *b = false;
        }
    }

    /// Returns the number of annotations set.
    pub fn count(&self) -> i32 {
        self.0.iter().fold(0, |n, &b| if b { n + 1 } else { n })
    }

    /// Returns the lowest annotation which is set.
    pub fn lowest(&self) -> Option<u8> {
        for (i, &b) in self.0.iter().enumerate() {
            if b {
                return Some(i as u8 + 1);
            }
        }
        None
    }

    /// Toggles the given annotation.
    pub fn toggle(&mut self, n: usize) {
        self.0[n - 1] = !self.0[n - 1];
    }
}

impl Debug for Annotations {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        for (n, &b) in self.0.iter().enumerate() {
            if b {
                write!(f, "{}", n + 1)?;
            }
        }
        Ok(())
    }
}

impl Default for Annotations {
    fn default() -> Self {
        Annotations::new()
    }
}

impl Index<u8> for Annotations {
    type Output = bool;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize - 1]
    }
}

impl IndexMut<u8> for Annotations {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.0[index as usize - 1]
    }
}

impl Sudoku {
    /// Creates a new `Sudoku` from the given 9x9 grid. Any entries besides 0-9 are invalid, with 0
    /// representing an empty cell.
    pub fn from_grid(grid: [[u8; 9]; 9]) -> Result<Self> {
        let mut s = Sudoku {
            grid,
            hints: [[Annotations::new(); 9]; 9],
            annotations: [[Annotations::new(); 9]; 9],
        };

        // Initialize hints array
        for i in 0..9 {
            for j in 0..9 {
                if s.grid[i][j] != 0 {
                    continue;
                }
                for n in 1..10 {
                    if s.is_valid_at(n, i, j) {
                        s.hints[i][j][n] = true;
                    }
                }
            }
        }

        Ok(s)
    }

    /// Returns whether the sudoku is solved.
    pub fn is_solved(&self) -> bool {
        for i in 0..9 {
            for j in 0..9 {
                if self.grid[i][j] == 0 || !self.is_valid_at(self.grid[i][j], i, j) {
                    return false;
                }
            }
        }
        true
    }

    /// Puts `n` at position `(row, col)`.
    pub fn put_at(&mut self, n: u8, row: usize, col: usize) {
        assert!(n <= 9);
        self.grid[row][col] = n;
        self.hints[row][col].clear();

        // Clear hints in 3x3 box
        let (boxrow, boxcol) = (row / 3 * 3, col / 3 * 3);
        for i in boxrow..boxrow + 3 {
            for j in boxcol..boxcol + 3 {
                self.hints[i][j][n] = false;
            }
        }
        // Clear hints in row and column
        for i in 0..9 {
            self.hints[row][i][n] = false;
            self.hints[i][col][n] = false;
        }
    }

    /// Returns an iterator over all solutions of this sudoku.
    pub fn solutions(&self) -> Solutions {
        Solutions { stack: vec![self.clone()] }
    }

    /// Returns the empty space which has the fewest hints (possibilities), or `None` if there are
    /// no empty spaces.
    fn find_min_poss(&self) -> Option<(usize, usize)> {
        let mut min = 10;
        let (mut row, mut col) = (0, 0);

        for i in 0..9 {
            for j in 0..9 {
                let cnt = self.hints[i][j].count();
                if self.grid[i][j] == 0 && cnt < min {
                    min = cnt;
                    row = i;
                    col = j;
                }
            }
        }

        if min == 10 { None } else { Some((row, col)) }
    }

    /// Returns whether the given entry number is valid at the given position.
    fn is_valid_at(&self, n: u8, row: usize, col: usize) -> bool {
        if n > 9 {
            return false;
        }

        // Check box
        let (boxrow, boxcol) = (row / 3 * 3, col / 3 * 3);
        for i in boxrow..boxrow + 3 {
            for j in boxcol..boxcol + 3 {
                if n == self.grid[i][j] && (i, j) != (row, col) {
                    return false;
                }
            }
        }
        // Check row and column
        for i in 0..9 {
            if n == self.grid[i][col] && i != row || n == self.grid[row][i] && i != col {
                return false;
            }
        }

        true
    }
}

impl Display for Sudoku {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        if f.alternate() {
            write!(f, "+---+---+---+\n")?;
        }
        for (i, row) in self.grid.iter().enumerate() {
            if f.alternate() {
                write!(f, "|")?;
            }
            for (j, col) in row.iter().enumerate() {
                write!(f, "{}", col)?;
                if f.alternate() && j % 3 == 2 {
                    write!(f, "|")?;
                }
            }
            if f.alternate() && i % 3 == 2 {
                write!(f, "\n+---+---+---+")?;
            }
            if i != 8 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl FromStr for Sudoku {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // We parse `.`, `0`, and `_` as empty squares, and ignore `|` characters and whitespace.
        let mut chars = s.chars().filter(|&c| !c.is_whitespace() && c != '|');
        let mut grid = [[0; 9]; 9];
        for i in 0..9 {
            for j in 0..9 {
                match chars.next() {
                    Some(c) => {
                        if c.is_digit(10) {
                            grid[i][j] = c as u8 - '0' as u8;
                        } else if c == '.' || c == '_' {
                            grid[i][j] = 0;
                        } else {
                            return Err(
                                ErrorKind::Parse(format!(
                                    "unexpected character `{}` at position ({}, {}) in sudoku",
                                    c,
                                    i,
                                    j
                                )).into(),
                            );
                        }
                    }
                    None => {
                        return Err(
                            ErrorKind::Parse(format!(
                                "unexpected end of input at position ({}, {}) in sudoku",
                                i,
                                j
                            )).into(),
                        )
                    }
                }
            }
        }

        Sudoku::from_grid(grid)
    }
}

impl Iterator for Solutions {
    type Item = Sudoku;

    fn next(&mut self) -> Option<Self::Item> {
        let mut s = match self.stack.pop() {
            // Stack empty; no more solutions to find
            None => return None,
            Some(s) => s,
        };
        // Find the position with the fewest possibilities
        let (row, col) = match s.find_min_poss() {
            None => {
                // No possibilities; need to backtrack
                if s.is_solved() {
                    return Some(s);
                } else {
                    return self.next();
                }
            }
            Some(p) => p,
        };

        // Try all the different possibilities until one works
        while let Some(n) = s.hints[row][col].lowest() {
            let mut tmp = s.clone();
            s.hints[row][col][n] = false;
            tmp.put_at(n, row, col);
            // Don't forget to put s back in the stack at its proper position
            self.stack.push(s);
            self.stack.push(tmp);

            return self.next();
        }
        // Ran out of possibilities; time to backtrack
        return self.next();
    }
}
