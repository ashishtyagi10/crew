//! The markdown file-viewer pane: a source|preview split over one file.
//! `mdpane_view` builds the `CellView`s (`cells`); this file owns the model,
//! the shared split geometry / scroll math both `cells` and `link_at` need,
//! and the click hit-test.
use std::path::PathBuf;

use crate::chatbody::CardLine;

/// Which half of the split is receiving input — Task 3 reads/toggles this to
/// route keys and mark the active side.
#[allow(dead_code)] // Task 2 wires MdPane into pane.rs; nothing constructs one yet
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Source,
    Preview,
}

/// One open markdown file: raw source plus independent scroll state for
/// each half of the split.
#[allow(dead_code)] // Task 2 wires MdPane into pane.rs; nothing constructs one yet
pub(crate) struct MdPane {
    pub path: PathBuf,
    pub source: String,
    pub active: Side,
    /// Rows the source half is scrolled down from its top, clamped to the
    /// last page in `cells`/`link_at` (constructing this doesn't yet know
    /// the pane's width, so it can't be clamped up front).
    pub scroll_src: usize,
    pub scroll_prev: usize,
}

#[allow(dead_code)] // Task 2 wires MdPane into pane.rs; nothing calls these yet
impl MdPane {
    /// Opens `path`'s already-read `source` at the top of both halves.
    pub(crate) fn new(path: PathBuf, source: String) -> Self {
        Self {
            path,
            source,
            active: Side::Source,
            scroll_src: 0,
            scroll_prev: 0,
        }
    }

    /// The URL under (`row`, `col`) on the preview half, or `None` on the
    /// source half, the divider, or empty space. Uses the exact same
    /// geometry and scroll windowing `cells` draws with, so a click always
    /// resolves the line it visibly sits on.
    pub(crate) fn link_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<String> {
        if cols == 0 || rows == 0 {
            return None;
        }
        let (_, _, right_start, right_w) = geometry(cols);
        if right_w == 0 || col < right_start {
            return None; // source side, divider column, or no room to render
        }
        let local_col = col - right_start;
        let lines = preview_lines(&self.source, right_w);
        let (start, end) = window_top(lines.len(), rows as usize, self.scroll_prev);
        let idx = start + row as usize;
        if idx >= end {
            return None;
        }
        crate::chatplace::cell_at_col(&lines[idx], local_col)
            .and_then(|cell| cell.link.as_deref())
            .map(str::to_string)
    }
}

/// Splits `cols` into `(left_width, divider_col, right_start, right_width)`.
/// `divider_col` always lands inside `0..cols` (never panics, even at
/// `cols == 1`, where the divider alone fills the pane and both halves are
/// zero-width).
#[allow(dead_code)] // only `mdpane_view`/tests call this until Task 2 wires the pane in
pub(crate) fn geometry(cols: u16) -> (usize, u16, u16, usize) {
    let left_w = (cols.saturating_sub(1) / 2) as usize;
    let divider_col = left_w as u16;
    let right_start = divider_col.saturating_add(1);
    let right_w = cols.saturating_sub(right_start) as usize;
    (left_w, divider_col, right_start, right_w)
}

/// Top-anchored scroll window: `(start, end)` indices into a `len`-line list
/// to show `rows` of it, `scroll` lines down from the top. Scrolling past
/// the end clamps to the last full page rather than running off the list.
#[allow(dead_code)] // only `mdpane_view`/tests call this until Task 2 wires the pane in
pub(crate) fn window_top(len: usize, rows: usize, scroll: usize) -> (usize, usize) {
    let max_start = len.saturating_sub(rows);
    let start = scroll.min(max_start);
    let end = (start + rows).min(len);
    (start, end)
}

/// `source` rendered through the shared markdown engine and mapped to card
/// lines exactly like chat bodies (`chatmd::map_lines`), at `right_w`
/// columns — the single place both `cells` and `link_at` read the preview
/// half from, so a click always resolves the same line it's drawn on.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent cell to
/// every line (matching the chat card layout it shares code with), so
/// content is wrapped one column narrower than `right_w`, same as
/// `chatbody::body_lines` does for chat cards — otherwise the last column of
/// every width-filling row would be clipped when `line_cells` draws at
/// `right_w` columns.
#[allow(dead_code)] // only `mdpane_view`/tests call this until Task 2 wires the pane in
pub(crate) fn preview_lines(source: &str, right_w: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    let content_w = right_w.saturating_sub(1);
    crate::chatmd::map_lines(crate::md::render(source, content_w), content_w, fg)
}

#[cfg(test)]
#[path = "mdpane_tests.rs"]
mod tests;
