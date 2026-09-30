//! A `sys:read_file` page's rows, each opened by the FILE's line number:
//! `  41│ text`, the form `sys:edit` shows an edited region in (`editrows`).
//!
//! WHY: a page was the file's text bare, with only its note saying which lines
//! it held (`lines 1,777–2,343 of 3,001`). Asked for line 1,777 of a saved run
//! of 3,000 random numbers, an agent read the page it opened there, counted
//! down a screen of look-alike rows to find its line, and answered with line
//! 2,333's. Claude Code's `Read` numbers every line, `cat -n` style, for
//! exactly this: the model reads the number beside the row instead of
//! counting. The numbers are the ones `sys:grep` reports and `{"line": N}`
//! opens at, and a row copied into `sys:edit`'s `old` with its number still on
//! is caught there (`editrows::carries_numbers`), the way one copied from an
//! edit's own rows is.
//!
//! A number costs bytes on every row, so this also chooses how much of the
//! file a page shows: whole lines while their NUMBERED rows fit the page's
//! budget, and the page's note then names the first byte not shown.
use std::fmt::Write;

#[cfg(test)]
#[path = "sysreadrows_tests.rs"]
mod tests;

/// Bytes a row adds to its line besides the number: `│` (three in UTF-8) and
/// the space after it.
const MARK: usize = 4;

fn digits(n: usize) -> usize {
    n.max(1).to_string().len()
}

/// The number of `text`'s last line, when its first is `first`: a trailing
/// newline closes the last line rather than opening another.
pub(super) fn last_line(text: &str, first: usize) -> usize {
    first + text.matches('\n').count() - usize::from(text.ends_with('\n'))
}

/// How many bytes of `text`, whose first line is `first`, a page shows so its
/// numbered rows take at most `budget` bytes: whole lines while they fit — all
/// of `text` when it does — or, when the first line alone does not, as much
/// of it as does, cut at a character. Only `text` empty shows nothing.
///
/// Costed exactly, at the width [`number`] will right-align to: that of the
/// last number shown, which every row pays when a line crosses into one more
/// digit.
pub(super) fn fit(text: &str, first: usize, budget: usize) -> usize {
    let (mut end, mut rows) = (0, 0);
    for line in text.split_inclusive('\n') {
        let cost = end + line.len() + (rows + 1) * (digits(first + rows) + MARK);
        if cost > budget {
            break;
        }
        end += line.len();
        rows += 1;
    }
    if end > 0 || text.is_empty() {
        return end;
    }
    // At least one character, so a page always moves the reader on.
    let room = budget.saturating_sub(digits(first) + MARK).min(text.len());
    (1..=room)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or_else(|| text.chars().next().map_or(0, char::len_utf8))
}

/// `text` with each line opened by its number, from `first`, right-aligned to
/// the widest one shown: `  9│ …` above ` 10│ …`. The line itself follows
/// byte for byte, its `\r` included, so taking the numbers off gives back
/// exactly what the file holds.
pub(super) fn number(text: &str, first: usize) -> String {
    if text.is_empty() {
        return String::new();
    }
    let w = digits(last_line(text, first));
    let rows = text.matches('\n').count() + 1;
    let mut out = String::with_capacity(text.len() + rows * (w + MARK));
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let _ = write!(out, "{:>w$}\u{2502} {line}", first + i);
    }
    out
}
