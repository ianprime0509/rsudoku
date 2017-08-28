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
//! A test of the RSudoku interactive game backend, which simulates a typical series of user
//! interactions with the game.

extern crate rsudoku;

use rsudoku::game::Game;
use rsudoku::Sudoku;

/// The example problem from Project Euler problem 96.
const EULER: &str = "003020600
900305001
001806400
008102900
700000008
006708200
002609500
800203009
005010300";

#[test]
fn interactive_game() {
    // The following steps represent a typical series of user interactions.
    // 1. Start the game
    let s = EULER.parse::<Sudoku>().unwrap();
    let mut game = Game::from_sudoku(s.clone());
    assert_eq!(game.board(), &s);
    assert_eq!(game.given(), &s);

    // 2. Try to insert a number in a given space, which should do nothing
    game.set_position(1, 0);
    let old = game.clone();
    game.put(4);
    assert_eq!(game, old, "inserting number in given space was not a no-op");

    // 3. Try to remove the number in that space, which should also do nothing
    game.remove();
    assert_eq!(game, old, "removing number in given space was not a no-op");

    // 4. Move to another space and insert a number there
    game.set_position(1, 6);
    let old = game.clone();
    game.put(8);
    assert_eq!(game.board()[1][6], 8);
    // This move is correct, so the board should still have a solution
    assert!(game.board().has_unique_solution());

    // 5. Undo that move
    game.undo();
    assert_eq!(game, old, "undo did not work correctly");

    // 6. Trying to undo further should do nothing
    assert_eq!(game.undo(), false, "was able to undo further than expected");

    // 7. Make a wrong move and then replace it with the right one.
    game.put(6);
    assert!(!game.board().has_unique_solution());
    game.put(8);
    assert!(game.board().has_unique_solution());

    // 8. Undo both moves
    game.undo();
    game.undo();
    assert_eq!(game, old, "undo did not work correctly");

    // 9. Use a hint
    game.hint().expect("could not give hint");
    assert!(game.board().has_unique_solution());

    // 10. Undo the hint
    game.undo();
    assert_eq!(game, old, "did not properly undo hint");

    // 11. Try moving around a bit
    game.set_position(0, 0);
    assert_eq!(game.position(), (0, 0));
    game.move_by(3, 5);
    assert_eq!(game.position(), (3, 5));
    game.move_by(6, 2);
    assert_eq!(game.position(), (3, 7));
    game.move_by(5, 1);
    assert_eq!(game.position(), (8, 8));

    // 12. Make sure that moving around wasn't captured in the undo history
    assert_eq!(game.undo(), false);

    // 13. Completely solve the game
    game.solve();
    assert!(game.is_solved());
    assert!(game.board().is_solved());

    // 14. Make sure the given board was never touched
    assert_eq!(game.given(), &s);

    // 15. Undo the solve action
    assert_eq!(game.undo(), true);
    game.set_position(1, 6);
    assert_eq!(game, old, "did not properly undo solve");
}
