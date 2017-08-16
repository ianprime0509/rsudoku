use std::default::Default;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter::{IntoIterator, Iterator};
use std::ops::{Index, IndexMut};
use std::str::FromStr;

use rand::{self, Rng};

use errors::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Annotations([bool; 9]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sudoku {
    grid: [[u8; 9]; 9],
    hints: [[Annotations; 9]; 9],
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

    /// Returns a `Vec` containing all the annotation numbers which are set.
    pub fn list(&self) -> Vec<u8> {
        self.0
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b)
            .map(|(i, _)| i as u8 + 1)
            .collect()
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
        for n in self.list() {
            write!(f, "{}", n)?;
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

impl<'a> IntoIterator for &'a Annotations {
    type Item = &'a bool;
    type IntoIter = <&'a Vec<bool> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Sudoku {
    /// Creates a new `Sudoku` from the given 9x9 grid. Any entries besides 0-9 are invalid, with 0
    /// representing an empty cell.
    pub fn from_grid(grid: [[u8; 9]; 9]) -> Result<Self> {
        let mut s = Sudoku {
            grid,
            hints: [[Annotations::new(); 9]; 9],
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

    /// Generates a `Sudoku` with a random grid. The generated `Sudoku` is guaranteed to have
    /// exactly one solution.
    pub fn generate() -> Self {
        // After generating a random, filled sudoku, we shuffle the positions of entries and try
        // removing them one by one. If, after removing an entry, we no longer have a solution,
        // then we put that entry back.
        let mut s = Sudoku::generate_filled();
        let mut positions = iproduct!(0..9, 0..9).collect::<Vec<_>>();
        rand::thread_rng().shuffle(positions.as_mut_slice());

        for (i, j) in positions {
            let removed = s.remove_at(i, j);
            if !s.has_unique_solution() {
                s.put_at(removed, i, j);
            }
        }

        assert!(s.has_unique_solution());
        s
    }

    /// Returns whether the `Sudoku` has a solution.
    pub fn has_solution(&self) -> bool {
        self.solutions().next().is_some()
    }

    /// Returns whether the `Sudoku` has a unique solution.
    pub fn has_unique_solution(&self) -> bool {
        self.solutions().take(2).count() == 1
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
    ///
    /// # Panics
    /// Will panic if `n` is not between 1 and 9, inclusive.
    pub fn put_at(&mut self, n: u8, row: usize, col: usize) {
        assert!(1 <= n && n <= 9, "entry number `{}` is invalid", n);

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

    /// Removes the entry at position `(row, col)`, returning the entry that was removed.
    pub fn remove_at(&mut self, row: usize, col: usize) -> u8 {
        let last = self.grid[row][col];
        self.grid[row][col] = 0;
        if last == 0 {
            return 0;
        }

        // Add this possibility back in the 3x3 box
        let (boxrow, boxcol) = (row / 3 * 3, col / 3 * 3);
        for i in boxrow..boxrow + 3 {
            for j in boxcol..boxcol + 3 {
                if self.is_valid_at(last, i, j) {
                    self.hints[i][j][last] = true;
                }
            }
        }
        // Add possibility in row and column
        for i in 0..9 {
            if self.is_valid_at(last, i, col) {
                self.hints[i][col][last] = true;
            }
            if self.is_valid_at(last, row, i) {
                self.hints[row][i][last] = true;
            }
        }
        // Recalculate possibilities for removed cell
        for n in 1..10 {
            if self.is_valid_at(n, row, col) {
                self.hints[row][col][n] = true;
            }
        }

        last
    }

    /// Returns an iterator over all solutions of this sudoku.
    pub fn solutions(&self) -> Solutions {
        Solutions { stack: vec![self.clone()] }
    }

    /// Generates a random, completely filled `Sudoku`.
    fn generate_filled() -> Self {
        // The process for generating a filled sudoku is to start with an empty grid. For each cell
        // in the grid, we try random possibilities for that cell until we find one such that the
        // sudoku still has a solution.
        let mut s = Sudoku::from_grid([[0; 9]; 9]).unwrap();

        for i in 0..9 {
            for j in 0..9 {
                let mut poss = s.hints[i][j].list();
                // We want to try possibilities randomly
                rand::thread_rng().shuffle(poss.as_mut_slice());
                for n in poss {
                    s.put_at(n, i, j);
                    if s.has_solution() {
                        break;
                    } else {
                        s.remove_at(i, j);
                    }
                }
            }
        }
        assert!(s.is_solved());
        s
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

impl Index<usize> for Sudoku {
    type Output = [u8; 9];

    fn index(&self, index: usize) -> &Self::Output {
        &self.grid[index]
    }
}

impl Iterator for Solutions {
    type Item = Sudoku;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(mut s) = self.stack.pop() {
            // Find the position with the fewest possibilities
            let (row, col) = match s.find_min_poss() {
                None => {
                    // No possibilities; need to backtrack
                    if s.is_solved() {
                        return Some(s);
                    } else {
                        continue;
                    }
                }
                Some(p) => p,
            };

            // Try one of the possibilities available at this position
            if let Some(n) = s.hints[row][col].lowest() {
                let mut tmp = s.clone();
                s.hints[row][col][n] = false;
                tmp.put_at(n, row, col);
                // Don't forget to put s back in the stack at its proper position
                self.stack.push(s);
                self.stack.push(tmp);
            }
        }
        // Nothing left on the stack
        None
    }
}
