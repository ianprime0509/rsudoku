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
//! Various utility functions.

/// Completes `cmd` using the given list (`commands`) of possible commands and returns a `String`
/// containing the common completion for `cmd`.
///
/// # Examples
///
/// ```
/// use rsudoku::util::complete;
///
/// assert_eq!("let",
///            complete(&["completion", "complete", "complet", "co", "com", "test"], "comp"));
/// ```
pub fn complete(commands: &[&str], cmd: &str) -> String {
    let possibilities = commands
        .iter()
        .filter(|&s| s.starts_with(cmd))
        .collect::<Vec<_>>();
    // Get the common beginning
    let cmdcnt = cmd.chars().count();

    if possibilities.is_empty() {
        String::new()
    } else {
        possibilities.iter().fold(
            possibilities[0][cmd.len()..].to_owned(),
            |acc, s| {
                acc.chars()
                    .zip(s.chars().skip(cmdcnt))
                    .take_while(|&(c1, c2)| c1 == c2)
                    .map(|(c1, _)| c1)
                    .collect()
            },
        )
    }
}
