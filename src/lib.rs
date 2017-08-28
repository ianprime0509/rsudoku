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
#[macro_use]
extern crate chan;
extern crate chan_signal;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate itertools;
extern crate rand;
extern crate termion;

pub mod errors {
    error_chain! {
        errors {
            /// The sudoku is invalid (has no solution).
            InvalidSudoku {
                description("sudoku is invalid")
            }
            /// An error encountered while parsing a `Sudoku`.
            Parse(s: String) {
                description("parse error")
                display("parse error: {}", s)
            }
        }
    }
}

pub mod game;
pub mod sudoku;
pub mod tui;
pub mod util;

pub use sudoku::Sudoku;

#[cfg(test)]
mod tests {
    use Sudoku;
    use sudoku::Annotations;

    /// Tests `Annotations`.
    #[test]
    fn annotations() {
        let mut a = Annotations::new();

        a[1] = true;
        assert_eq!(
            a,
            Annotations::from_array(
                [true, false, false, false, false, false, false, false, false],
            )
        );

        a.toggle(9);
        assert_eq!(
            a,
            Annotations::from_array(
                [true, false, false, false, false, false, false, false, true],
            )
        );

        a.toggle(1);
        assert_eq!(
            a,
            Annotations::from_array(
                [false, false, false, false, false, false, false, false, true],
            )
        );

        assert_eq!(a[9], true);

        a = Annotations::from_array([true, false, true, false, true, false, true, true, false]);
        assert_eq!(a.count(), 5);

        a.clear();
        assert_eq!(a, Annotations::new());
    }

    /// Tests sudoku parsing.
    #[test]
    fn parse() {
        let input = "003020600
900305001
001806400
008102900
700000008
006708200
002609500
800203009
005010300";
        let s = input.parse::<Sudoku>().expect("could not parse sudoku");

        assert_eq!(
            s,
            Sudoku::from_grid(
                [
                    [0, 0, 3, 0, 2, 0, 6, 0, 0],
                    [9, 0, 0, 3, 0, 5, 0, 0, 1],
                    [0, 0, 1, 8, 0, 6, 4, 0, 0],
                    [0, 0, 8, 1, 0, 2, 9, 0, 0],
                    [7, 0, 0, 0, 0, 0, 0, 0, 8],
                    [0, 0, 6, 7, 0, 8, 2, 0, 0],
                    [0, 0, 2, 6, 0, 9, 5, 0, 0],
                    [8, 0, 0, 2, 0, 3, 0, 0, 9],
                    [0, 0, 5, 0, 1, 0, 3, 0, 0],
                ],
            ).unwrap()
        );
    }

    /// Tests whether sudoku parsing will fail on invalid input (as it should).
    #[test]
    fn parse_invalid() {
        let input1 = "000000000
000000000
000900900
000000000
000000000
000000000
000000000
000000000
000000000";
        assert!(input1.parse::<Sudoku>().is_err());

        let input2 = "000000000
000009000
000900000
000000000
000000000
000000000
000000000
000000000
000000000";
        assert!(input2.parse::<Sudoku>().is_err());

        let input3 = "000000000
000000000
000900000
000000000
000000000
000000000
000000000
000900000
000000000";
        assert!(input3.parse::<Sudoku>().is_err());

        let input4 = "This is not a Sudoku!";
        assert!(input4.parse::<Sudoku>().is_err());

        let input5 = "000000000
000000000
000900000
000000000
000000000
000000000
000000000
000900000
000000000
trailing output is bad";
        assert!(input5.parse::<Sudoku>().is_err());
    }

    /// Tests sudoku generation.
    #[test]
    fn generate() {
        // Make sure that generated sudokus have precisely one solution
        for _ in 1..10 {
            let s = Sudoku::generate();
            assert!(s.has_unique_solution());
        }
    }
}
