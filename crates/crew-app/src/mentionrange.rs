//! Line ranges on a file mention: `@src/main.rs:120-180`.
//!
//! `@file` attaches the whole file, and a file past `MAX_FILE_BYTES` is
//! skipped entirely — so the case where you most want to point an agent at
//! one function (a big module) was the case you could not point it at at all.
//! A range makes that file usable, and makes an ordinary one cheaper: the
//! agents read forty lines instead of two thousand.
//!
//! `:120` is line 120. `:120-180` is 120 through 180, inclusive, counting
//! from 1 — the numbering every editor and every stack trace uses.

/// A mention's `:start[-end]` suffix, as 1-based inclusive line numbers.
pub(crate) type Range = (usize, usize);

/// Split `@`-token text into its path and its line range.
///
/// A range only exists when the suffix after the LAST colon is digits (or
/// digits-hyphen-digits) and the path in front of it is non-empty. Anything
/// else is part of the name: a colon is a legal character in a filename, and
/// silently truncating `notes:draft.md` to `notes` would be worse than not
/// having ranges at all. The caller checks the whole token as a path first,
/// so a file genuinely named `x:10` still wins.
pub(crate) fn split(rel: &str) -> (&str, Option<Range>) {
    let Some(colon) = rel.rfind(':') else {
        return (rel, None);
    };
    let (path, suffix) = (&rel[..colon], &rel[colon + 1..]);
    if path.is_empty() {
        return (rel, None);
    }
    match parse_range(suffix) {
        Some(r) => (path, Some(r)),
        None => (rel, None),
    }
}

/// `120` → `(120, 120)`; `120-180` → `(120, 180)`. `None` for anything that
/// is not a line number, and for a backwards or zero range — line 0 does not
/// exist, and `40-10` is a typo rather than an empty selection.
fn parse_range(s: &str) -> Option<Range> {
    let (a, b) = match s.split_once('-') {
        Some((a, b)) => (a, b),
        None => (s, s),
    };
    let start: usize = a.parse().ok()?;
    let end: usize = b.parse().ok()?;
    (start >= 1 && end >= start).then_some((start, end))
}

/// The selected lines of `text`, or `None` when the range starts past its
/// end. `end` beyond the last line simply stops there — asking for 100-200 of
/// a 150-line file is a reasonable way to say "to the end".
pub(crate) fn slice(text: &str, (start, end): Range) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if start > lines.len() {
        return None;
    }
    Some(lines[start - 1..end.min(lines.len())].join("\n"))
}

/// How a range reads in the attachment header: `lines 120-180`, or
/// `line 120` when it is one.
pub(crate) fn label((start, end): Range) -> String {
    if start == end {
        format!("line {start}")
    } else {
        format!("lines {start}-{end}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_and_a_single_line_both_parse() {
        assert_eq!(
            split("src/main.rs:120-180"),
            ("src/main.rs", Some((120, 180)))
        );
        assert_eq!(split("src/main.rs:120"), ("src/main.rs", Some((120, 120))));
    }

    #[test]
    fn a_plain_path_has_no_range() {
        assert_eq!(split("src/main.rs"), ("src/main.rs", None));
        assert_eq!(split("README"), ("README", None));
    }

    /// A colon is a legal character in a filename. Truncating a name to the
    /// part before it would lose the file entirely, which is worse than
    /// having no ranges.
    #[test]
    fn a_colon_that_is_not_a_range_stays_in_the_name() {
        assert_eq!(split("notes:draft.md"), ("notes:draft.md", None));
        assert_eq!(split("a:b:c"), ("a:b:c", None));
        assert_eq!(split(":40"), (":40", None), "no path in front of it");
        assert_eq!(split("x:"), ("x:", None), "no numbers behind it");
    }

    /// `skill:` mentions are handled before this is ever reached, but a name
    /// that merely looks like one must not acquire a range either.
    #[test]
    fn a_word_suffix_is_never_a_range() {
        assert_eq!(split("skill:review"), ("skill:review", None));
        assert_eq!(split("host:8080x"), ("host:8080x", None));
    }

    #[test]
    fn nonsense_ranges_are_not_ranges() {
        assert_eq!(split("f.rs:0"), ("f.rs:0", None), "there is no line 0");
        assert_eq!(split("f.rs:40-10"), ("f.rs:40-10", None), "backwards");
        assert_eq!(split("f.rs:-5"), ("f.rs:-5", None));
    }

    #[test]
    fn slicing_is_one_based_and_inclusive() {
        let text = "one\ntwo\nthree\nfour\nfive";
        assert_eq!(slice(text, (2, 4)).unwrap(), "two\nthree\nfour");
        assert_eq!(slice(text, (1, 1)).unwrap(), "one");
        assert_eq!(slice(text, (5, 5)).unwrap(), "five");
    }

    /// Asking past the end is a reasonable way to say "to the end"; asking
    /// for a start past the end selects nothing, and must say so rather than
    /// silently attaching an empty block.
    #[test]
    fn an_end_past_the_file_stops_there_and_a_start_past_it_is_nothing() {
        let text = "one\ntwo\nthree";
        assert_eq!(slice(text, (2, 900)).unwrap(), "two\nthree");
        assert_eq!(slice(text, (4, 9)), None);
        assert_eq!(slice("", (1, 5)), None);
    }

    #[test]
    fn the_label_reads_as_one_line_or_several() {
        assert_eq!(label((120, 180)), "lines 120-180");
        assert_eq!(label((7, 7)), "line 7");
    }
}
