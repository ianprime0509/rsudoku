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
