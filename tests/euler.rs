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
