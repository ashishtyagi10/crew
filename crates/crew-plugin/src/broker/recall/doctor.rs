//! `/doctor`'s recall line, apart from the graph it reports on.
use crate::broker::wording::count;

/// `/doctor`'s line: the mark and the detail.
pub(crate) fn doctor_line(
    turns: usize,
    topics: usize,
    files: usize,
    pages: usize,
) -> (char, String) {
    let mark = if turns > 0 { '\u{2713}' } else { '\u{2013}' };
    // Pages are named only once there are some. Every project has files; a
    // project that has never sent crew to the web should not read as one
    // missing something.
    let pages = match pages {
        0 => String::new(),
        n => format!(", {}", count(n, "page")),
    };
    (
        mark,
        format!(
            "{}, {}, {}{pages} in the recall graph",
            count(turns, "turn"),
            count(topics, "topic"),
            count(files, "file")
        ),
    )
}
