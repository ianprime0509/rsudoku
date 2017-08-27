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
extern crate rsudoku;

use rsudoku::Sudoku;

const PROBLEMS: &str = include_str!("euler96.txt");
const SOLUTIONS: &str = include_str!("euler96solutions.txt");

/// Checks the sols to the sudokus given in Project Euler problem 96.
#[test]
fn solve_euler96() {
    let probs = read(PROBLEMS);
    let sols = read(SOLUTIONS);

    assert_eq!(probs.len(), sols.len());
    for (n, (p, s)) in probs.iter().zip(sols.iter()).enumerate() {
        let psols = p.solutions().collect::<Vec<_>>();
        assert_eq!(
            psols.len(),
            1,
            "problem {} yielded {} solutions",
            n,
            psols.len()
        );
        assert_eq!(&psols[0], s);
    }
}

fn read(probs: &str) -> Vec<Sudoku> {
    probs
        .split("=========")
        .map(|s| s.parse().expect("could not parse input"))
        .collect()
}
