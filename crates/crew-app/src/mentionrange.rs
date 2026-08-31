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
#[path = "mentionrange_tests.rs"]
mod tests;
