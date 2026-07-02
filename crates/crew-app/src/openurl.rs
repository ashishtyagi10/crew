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

impl CrewApp {
    /// The row text and character column under the cursor in a terminal pane
    /// (content rows only; the title bar is excluded). Drives Cmd+click.
    pub(crate) fn cursor_cell(&self) -> Option<(String, usize)> {
        let i = self.pane_at_cursor()?;
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
