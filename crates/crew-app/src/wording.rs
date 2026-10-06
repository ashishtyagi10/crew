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

/// `n` and a noun whose plural is not just `+s`: `1 entry`, `3 entries`.
pub(crate) fn count_as(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Items as a sentence lists them: `#3`, `#3 and #4`, `#3, #4 and #5`.
pub(crate) fn series(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [head @ .., last] => format!("{} and {last}", head.join(", ")),
    }
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
    fn a_series_reads_as_a_sentence_lists_it() {
        let s = |v: &[&str]| super::series(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        assert_eq!(s(&["#3"]), "#3");
        assert_eq!(s(&["#3", "#4"]), "#3 and #4");
        assert_eq!(s(&["#3", "#4", "#5"]), "#3, #4 and #5");
    }

    #[test]
    fn an_irregular_plural_is_named_whole() {
        assert_eq!(super::count_as(1, "entry", "entries"), "1 entry");
        assert_eq!(super::count_as(0, "entry", "entries"), "0 entries");
        assert_eq!(super::count_as(7, "entry", "entries"), "7 entries");
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
