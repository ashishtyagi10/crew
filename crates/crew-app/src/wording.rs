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

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn one_is_singular_and_everything_else_is_not() {
        assert_eq!(count(1, "pane"), "1 pane");
        assert_eq!(count(0, "pane"), "0 panes");
        assert_eq!(count(2, "message"), "2 messages");
    }
}
