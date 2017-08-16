#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate itertools;
extern crate rand;

pub mod errors {
    error_chain! {
        errors {
            /// An error encountered while parsing a `Sudoku`.
            Parse(s: String) {
                description("parse error")
                display("parse error: {}", s)
            }
        }
    }
}

pub mod sudoku;

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

        assert_eq!(a[2], false);

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
