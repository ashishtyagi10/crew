//! What Cmd+E can label, read by LOGICAL line.
//!
//! A path longer than the pane is drawn across two rows, and read row by
//! row it was two things: `crates/crew-app/src/md/` on one, `fold.rs:25` on
//! the next — and the label went on the second, which opened a `fold.rs`
//! that is not there. The terminal says which rows soft-wrap onto the next
//! (`TermModel::wrapped_rows`); those rows are joined before anything is
//! looked for, and each target is placed back at the cell it starts in.
use crate::hints::Kind;

/// The URLs, paths and hashes on one line, left to right, never overlapping:
/// a URL outranks the path its slashes also make, and both outrank a hash.
fn line_spans(line: &[char]) -> Vec<(usize, usize, Kind)> {
    let mut spans: Vec<(usize, usize, Kind)> = crate::openurl::url_spans(line)
        .into_iter()
        .map(|(a, b)| (a, b, Kind::Url))
        .collect();
    let free = |spans: &[(usize, usize, Kind)], a: usize, b: usize| {
        !spans.iter().any(|&(x, y, _)| a < y && x < b)
    };
    for (a, b) in crate::pathhl::path_spans(line) {
        if free(&spans, a, b) {
            spans.push((a, b, Kind::Path));
        }
    }
    for (a, b) in crate::hints::hash_spans(line) {
        if free(&spans, a, b) {
            spans.push((a, b, Kind::Hash));
        }
    }
    spans.sort_by_key(|&(a, _, _)| a);
    spans
}

/// Every target on `rows` as `(row, col, text, kind)`, `row`/`col` being the
/// cell it starts in. `wraps[r]` says row `r` continues on row `r + 1`; a
/// missing flag is a hard line end.
pub(crate) fn found(rows: &[Vec<char>], wraps: &[bool]) -> Vec<(u16, u16, String, Kind)> {
    let mut out = Vec::new();
    let mut r = 0;
    while r < rows.len() {
        let mut end = r;
        while end + 1 < rows.len() && wraps.get(end).copied().unwrap_or(false) {
            end += 1;
        }
        let mut line: Vec<char> = Vec::new();
        let mut starts: Vec<usize> = Vec::new();
        for row in &rows[r..=end] {
            starts.push(line.len());
            line.extend(row);
        }
        for (a, b, kind) in line_spans(&line) {
            let k = starts.partition_point(|&s| s <= a) - 1;
            let text: String = line[a..b].iter().collect();
            out.push(((r + k) as u16, (a - starts[k]) as u16, text, kind));
        }
        r = end + 1;
    }
    out
}

#[cfg(test)]
#[path = "hintscan_tests.rs"]
mod tests;
