//! The one place a count is put into words.
//!
//! `1 call(s)`, `close all 1 panes?`, `1 message(s)`: three surfaces, three
//! ways of not deciding. Eighteen other sites already pick the ending from
//! the number; this is that choice, written once, so a listing and the
//! status line under it cannot disagree about one thing.

/// `n` and its noun: `1 call`, `3 calls`.
pub(crate) fn count(n: usize, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

/// `copied 1 line{tail}` / `copied 12 lines{tail}` — the copy confirmations,
/// which said `copied 1 lines`.
pub(crate) fn copied(lines: usize, tail: &str) -> String {
    format!("copied {}{tail}", count(lines, "line"))
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn one_is_singular_and_everything_else_is_not() {
        assert_eq!(count(1, "pane"), "1 pane");
        assert_eq!(count(0, "pane"), "0 panes");
        assert_eq!(count(2, "message"), "2 messages");
    }

    #[test]
    fn a_copy_of_one_line_is_one_line() {
        assert_eq!(super::copied(1, ""), "copied 1 line");
        assert_eq!(
            super::copied(12, " (scrollback)"),
            "copied 12 lines (scrollback)"
        );
    }
}
