//! Two edits that are spelled in markdown rather than in the render, because the render does not
//! show what they act on.
//!
//! A link's URL is invisible — that is what rendering a link means — and a table's cell walls are
//! drawn, not typed, so `Tab` has nothing on screen to aim at. Both are answered the same way the
//! rest of the editor is: by working on the SOURCE at a byte offset, which the caret already is.
//!
//! Everything here is a pure function of `(source, byte)`. No pane, no cells, no layout — so the
//! cases that matter (a link at the very end of a file, a table row with a trailing pipe, an
//! unclosed bracket) are tested as arithmetic rather than as a window.

/// Where an inline link lives in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinkSpan {
    /// The whole `[text](url)`, or `![alt](url)` including the `!`.
    pub whole: (u32, u32),
    /// Just the URL, which is what an edit replaces.
    pub url: (u32, u32),
}

/// The inline link containing byte `at`, if there is one.
///
/// Deliberately literal: it finds `](`, walks back to the `[` that opens it and forward to the
/// `)` that closes it, counting parenthesis depth so a URL with brackets in it survives. A
/// reference-style link (`[text][ref]`) is not one of these and returns `None` rather than
/// guessing at a definition somewhere else in the file.
pub(crate) fn link_at(src: &str, at: u32) -> Option<LinkSpan> {
    let b = src.as_bytes();
    let at = at as usize;
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] != b']' || b[i + 1] != b'(' {
            i += 1;
            continue;
        }
        let Some(open) = back_to_bracket(b, i) else {
            i += 1;
            continue;
        };
        let Some(close) = forward_to_paren(b, i + 1) else {
            i += 1;
            continue;
        };
        // The `!` of an image belongs to the link: deleting or replacing without it leaves a
        // stray bang in the file.
        let start = match open > 0 && b[open - 1] == b'!' {
            true => open - 1,
            false => open,
        };
        if (start..=close).contains(&at) {
            return Some(LinkSpan {
                whole: (start as u32, close as u32 + 1),
                url: (i as u32 + 2, close as u32),
            });
        }
        i = close + 1;
    }
    None
}

/// The `[` opening the link that `close` (`]`) ends, on the same line.
fn back_to_bracket(b: &[u8], close: usize) -> Option<usize> {
    let mut i = close;
    while i > 0 {
        i -= 1;
        match b[i] {
            b'[' => return Some(i),
            // A link's text does not span lines; running past one means this `]` opened nothing.
            b'\n' => return None,
            _ => {}
        }
    }
    None
}

/// The `)` closing the `(` at `open`, counting depth.
fn forward_to_paren(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'\n' => return None,
            _ => {}
        }
    }
    None
}

/// The byte a `Tab` should put the caret on: the start of the next table cell.
///
/// `None` when the caret is not in a table row, which is the signal to type a Tab's worth of
/// spaces instead. Walking off the last cell of a row lands on the first cell of the next row,
/// skipping the `|---|` divider — the divider is punctuation, not a cell anybody types in.
pub(crate) fn next_cell(src: &str, at: u32) -> Option<u32> {
    let at = at as usize;
    let (start, line) = line_at(src, at)?;
    if !is_row(line) {
        return None;
    }
    if let Some(next) = cell_after(line, at - start) {
        return Some((start + next) as u32);
    }
    // Off the end of this row: the first cell of the next row that is not a divider.
    let mut cursor = start + line.len() + 1;
    while let Some((next_start, next_line)) = line_at(src, cursor) {
        if !is_row(next_line) {
            return None;
        }
        if !is_divider(next_line) {
            return first_cell(next_line).map(|c| (next_start + c) as u32);
        }
        cursor = next_start + next_line.len() + 1;
    }
    None
}

/// The line containing byte `at`, as (its start, its text without the newline).
fn line_at(src: &str, at: usize) -> Option<(usize, &str)> {
    if at > src.len() {
        return None;
    }
    let start = src[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = src[start..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(src.len());
    Some((start, &src[start..end]))
}

/// A table row: a line whose first non-space character is a pipe.
fn is_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// The `|---|---|` line under a header, which is not a cell to type in.
fn is_divider(line: &str) -> bool {
    line.trim()
        .trim_matches('|')
        .split('|')
        .all(|c| !c.trim().is_empty() && c.trim().chars().all(|ch| ch == '-' || ch == ':'))
}

/// The start of a row's FIRST cell: past the leading pipe and its padding.
fn first_cell(line: &str) -> Option<usize> {
    let pipe = line.find('|')?;
    past_padding(line, pipe)
}

/// The start of the cell after byte `from` within `line`, skipping the pipe and the padding
/// space people write after it. `None` when there is no further cell — a trailing `|` at the end
/// of the row closes the last cell rather than opening a new one.
fn cell_after(line: &str, from: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let pipe = bytes.iter().skip(from + 1).position(|c| *c == b'|')? + from + 1;
    past_padding(line, pipe)
}

/// Just past the pipe at `pipe` and the space people write after it — a caret on the padding
/// would read as being in the previous cell's gutter.
fn past_padding(line: &str, pipe: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = pipe + 1;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    (i < bytes.len()).then_some(i)
}

impl super::ViewPane {
    /// Tab: put the caret on the next table cell. `false` when the caret is not in a table, and
    /// the caller types a tab's worth of spaces instead.
    pub(crate) fn tab_cell(&mut self, cols: u16, rows: u16) -> bool {
        let Some(at) = self.caret_at else {
            return false;
        };
        let Some(to) = self.source_str().and_then(|s| next_cell(s, at)) else {
            return false;
        };
        self.clear_selection();
        self.history.breaks();
        self.after_edit(to as usize, cols, rows);
        true
    }

    /// The link the caret is inside, in source bytes.
    pub(crate) fn caret_link_span(&self) -> Option<LinkSpan> {
        link_at(self.source_str()?, self.caret_at?)
    }
}

#[cfg(test)]
#[path = "mdedit_tests.rs"]
mod tests;
