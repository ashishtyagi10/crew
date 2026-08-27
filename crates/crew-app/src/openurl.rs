//! URL detection in terminal rows: powers the blue link tint (`linkhl`) and
//! Cmd+click resolution (`clickopen`) — a clicked URL opens in the browser.
use crate::app::CrewApp;
use crate::dump::grid_row;
use crate::pane::PaneContent;
use crew_term::TermModel;

/// Characters trimmed from a URL's tail (trailing punctuation in prose).
const TRAILERS: &str = ".,);]}>\"'";

/// Character spans `[start, end)` of the http(s) URLs in `chars` (one row of a
/// terminal grid). Trailing prose punctuation is excluded from each span.
pub(crate) fn url_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let tail: String = chars[i..].iter().take(8).collect();
        if tail.starts_with("http://") || tail.starts_with("https://") {
            let mut j = i;
            while j < chars.len() && !chars[j].is_whitespace() {
                j += 1;
            }
            let mut end = j;
            while end > i && TRAILERS.contains(chars[end - 1]) {
                end -= 1;
            }
            if end - i > "https://".len() {
                spans.push((i, end));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    spans
}

/// Returns the http(s) URL spanning character column `col` in `line`, if `col`
/// falls inside one. Used to resolve a Cmd+click to a link.
pub(crate) fn url_at(line: &str, col: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    url_spans(&chars)
        .into_iter()
        .find(|&(a, b)| (a..b).contains(&col))
        .map(|(a, b)| chars[a..b].iter().collect())
}

/// Schemes a clicked OSC 8 hyperlink may open.
///
/// The target of an OSC 8 link is chosen by whatever program is writing to the
/// pane, not by the person clicking, and the visible text says nothing about
/// it — so a click on the word "docs" would otherwise hand an arbitrary URL
/// scheme to the system opener. These four are the ones a document can
/// reasonably mean; anything else is shown and refused.
const SAFE_SCHEMES: [&str; 4] = ["http://", "https://", "mailto:", "file://"];

/// The hyperlink target if crew is willing to open it. Scheme matching is
/// case-insensitive — `HTTPS://` is a URL, and is also how a scheme filter
/// gets slipped past.
pub(crate) fn safe_link(uri: &str) -> Option<&str> {
    let lower = uri.to_ascii_lowercase();
    SAFE_SCHEMES
        .iter()
        .any(|s| lower.starts_with(s))
        .then_some(uri)
}

impl CrewApp {
    /// The OSC 8 hyperlink target under the cursor, if the terminal pane there
    /// has one. Separate from `cursor_cell`: this is what the *program* said
    /// the text points at, which the text itself need not resemble.
    pub(crate) fn cursor_link(&self) -> Option<String> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        let PaneContent::Terminal(t) = &self.panes[i].content else {
            return None;
        };
        t.pty.link_at(col.try_into().ok()?, row.try_into().ok()?)
    }

    /// The `(row, col)` content-grid cell under the cursor in pane `i`'s rect
    /// (content rows only; the title bar is excluded). Shared pixel→cell math
    /// for both the terminal Cmd+click path (`cursor_cell`, below) and the
    /// chat-pane link hit-test in `clickopen`.
    pub(crate) fn cursor_rowcol(&self, i: usize) -> Option<(i32, i32)> {
        let (cw, ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let rect = self
            .pane_hit_rects()
            .into_iter()
            .find(|&(idx, _)| idx == i)
            .map(|(_, r)| r)?;
        let col = ((self.cursor.0 - rect.x) / cw).floor() as i32;
        // Content sits one row below the pane's title bar.
        let row = ((self.cursor.1 - rect.y) / ch).floor() as i32 - 1;
        if col < 0 || row < 0 {
            return None;
        }
        Some((row, col))
    }

    /// The row text and character column under the cursor in a terminal pane.
    /// Drives Cmd+click.
    pub(crate) fn cursor_cell(&self) -> Option<(String, usize)> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        let pane = &self.panes[i];
        let PaneContent::Terminal(t) = &pane.content else {
            return None;
        };
        let line = grid_row(&t.pty.cells(false), row as u16, pane.grid.cols);
        Some((line, col as usize))
    }
}

#[cfg(test)]
#[path = "openurl_tests.rs"]
mod tests;
