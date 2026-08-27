//! What an error line looks like, across the tools that print them.
//!
//! A long build scrolls its own failure off the screen, and finding it again
//! means either remembering a word from it or paging up through the noise.
//! `/errors` walks back to the most recent one instead — which needs a
//! definition of "one" that holds for `rustc`, `tsc`, `gcc`, a Python
//! traceback, `npm`, and the test runners, without firing on every line that
//! merely contains the word.
//!
//! The rule: an error announces itself at the START of a line, or right after
//! a `file:line:col` prefix. Prose that mentions errors does neither.

/// Markers that make a line an error when it LEADS the line (after any
/// indent, quote bar or box-drawing chrome a TUI drew around it).
const LEADING: [&str; 12] = [
    "error",
    "fatal:",
    "fatal error",
    "panicked at",
    "thread 'main' panicked",
    "failed",
    "failure:",
    "npm err!",
    "traceback (most recent call last)",
    "not ok ",
    "\u{2717}", // ✗ — what most test runners mark a failure with
    "\u{2718}",
];

/// Markers that make a line an error wherever they appear, because they carry
/// their own punctuation: a compiler's `path:12:3: error:` prefix, or a
/// diagnostic code.
const ANYWHERE: [&str; 4] = [": error", ": fatal error", "error[", "error ts"];

/// Trim the chrome a TUI or a log format puts in front of a line: indent,
/// quote bars, bullets, box edges, and a leading timestamp's brackets.
fn strip_chrome(line: &str) -> &str {
    line.trim_start_matches(|c: char| {
        c.is_whitespace() || "|>\u{2502}\u{2503}\u{2551}*-\u{2022}\u{25b8}\u{276f}".contains(c)
    })
}

/// Whether `line` reads as an error some tool printed.
pub(crate) fn looks_like_error(line: &str) -> bool {
    let head = strip_chrome(line).to_lowercase();
    if head.is_empty() {
        return false;
    }
    if LEADING.iter().any(|m| head.starts_with(m)) {
        // `error` on its own leads a diagnostic; `errors are handled here` is
        // a sentence. The character after the word is what tells them apart.
        return !head.starts_with("error")
            || head[5..]
                .chars()
                .next()
                .is_none_or(|c| ":[ \u{2014}".contains(c));
    }
    ANYWHERE.iter().any(|m| head.contains(m))
}

/// Which visible rows of a pane hold an error, for the marks its card draws
/// down the left border.
///
/// The border rather than the content: a terminal's columns belong to the
/// program running in it, and a marker in column zero would overwrite the
/// first character of whatever the error line says.
pub(crate) fn error_rows(lines: &[Vec<char>]) -> Vec<u16> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| looks_like_error(&l.iter().collect::<String>()))
        .map(|(i, _)| i as u16)
        .collect()
}

#[cfg(test)]
#[path = "errscan_tests.rs"]
mod tests;
