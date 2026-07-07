//! The markdown file-viewer pane: a source|preview split over one file.
//! `mdpane_view` builds the `CellView`s (`cells`); this file owns the model,
//! the shared split geometry / scroll math both `cells` and `link_at` need,
//! and the click hit-test.
use std::path::PathBuf;

use winit::event::KeyEvent;

use crate::chatbody::CardLine;
use crate::mdkeys::{self, MdAction};

/// Which half of the split is receiving input — Tab (`mdkeys::reduce`)
/// toggles this and it routes scrolling and marks the active-side indicator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Source,
    Preview,
}

impl Side {
    /// The side Tab switches to.
    pub(crate) fn other(self) -> Side {
        match self {
            Side::Source => Side::Preview,
            Side::Preview => Side::Source,
        }
    }
}

/// One open markdown file: raw source plus independent scroll state for
/// each half of the split.
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
    #[allow(dead_code)] // Task 4 wires this into clickopen.rs
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

    /// Handle a winit key event: Tab flips the active side, arrows/PageUp/
    /// PageDown scroll it, `r` reloads `path` from disk, Esc asks the host to
    /// close the pane. Decoding + effects live in `mdkeys` as a pure,
    /// testable seam — mirrors `ChatPane::on_key`/`FarPane::on_key`.
    pub(crate) fn on_key(&mut self, key: &KeyEvent) -> Option<MdAction> {
        let input = mdkeys::md_key(&key.logical_key, key.state.is_pressed());
        mdkeys::reduce(self, input)
    }

    /// Scrolls one half of the split by `delta` lines, floored at the top —
    /// `window_top` clamps the far end at render time, so this only needs to
    /// keep the offset from going negative. Convention matches
    /// `FarPane::scroll`/`ChatPane::scroll`: positive `delta` moves toward
    /// the top/start of the content.
    pub(crate) fn scroll(&mut self, side: Side, delta: i32) {
        let target = match side {
            Side::Source => &mut self.scroll_src,
            Side::Preview => &mut self.scroll_prev,
        };
        *target = (*target as i64 - delta as i64).max(0) as usize;
    }

    /// Routes a mouse-wheel scroll to whichever half `cursor_col` sits over
    /// (source left of the divider, preview at/right of `right_start`).
    /// `None` — a keyboard-triggered scroll with no cursor position, e.g.
    /// Shift+PageUp/Home/End — falls back to the active side.
    pub(crate) fn scroll_wheel(&mut self, cols: u16, cursor_col: Option<u16>, delta: i32) {
        let (_, _, right_start, right_w) = geometry(cols);
        let side = match cursor_col {
            Some(c) if right_w > 0 && c >= right_start => Side::Preview,
            Some(_) => Side::Source,
            None => self.active,
        };
        self.scroll(side, delta);
    }

    /// Re-reads `path` from disk, replacing `source` on success. On failure
    /// (missing file, permissions, non-UTF-8) the old content is left in
    /// place and an error message comes back for the caller to show as a
    /// status line — same wording as `spawn_md_pane`'s initial-read error.
    pub(crate) fn reload(&mut self) -> Result<(), String> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => {
                self.source = s;
                Ok(())
            }
            Err(e) => Err(format!("md: cannot read {}: {e}", self.path.display())),
        }
    }
}

/// Splits `cols` into `(left_width, divider_col, right_start, right_width)`.
/// `divider_col` always lands inside `0..cols` (never panics, even at
/// `cols == 1`, where the divider alone fills the pane and both halves are
/// zero-width).
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
pub(crate) fn preview_lines(source: &str, right_w: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    let content_w = right_w.saturating_sub(1);
    crate::chatmd::map_lines(crate::md::render(source, content_w), content_w, fg)
}

#[cfg(test)]
#[path = "mdpane_tests.rs"]
mod tests;
