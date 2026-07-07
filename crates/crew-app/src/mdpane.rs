//! The markdown file-viewer pane: a source|preview split over one file.
//! `mdpane_view` builds the `CellView`s (`cells`); `mdcache` precomputes the
//! wrapped-source/preview lines both halves render from; this file owns the
//! model, the shared split geometry / scroll math, and the click hit-test.
use std::cell::RefCell;
use std::path::PathBuf;

use winit::event::KeyEvent;

use crate::mdcache::MdCache;
use crate::mdkeys::{self, MdAction};
#[cfg(test)]
use std::cell::Cell;

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
    /// Precomputed wrapped-source/preview lines for the last `cols` this
    /// pane rendered at (see `mdcache::cache_for`); `None` before the first
    /// render or right after a `reload`. `RefCell` because `cells`/`link_at`
    /// only need a shared `&self` to draw/hit-test — this is the one piece
    /// of mutable state that has to live behind interior mutability instead.
    /// `pub(crate)` so `mdcache`'s `impl MdPane` block can reach it.
    pub(crate) cache: RefCell<Option<MdCache>>,
    /// Counts cache rebuilds so tests can pin "the cache is reused, not
    /// rebuilt every call" without depending on wall-clock timing.
    #[cfg(test)]
    pub(crate) rebuilds: Cell<u32>,
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
            cache: RefCell::new(None),
            #[cfg(test)]
            rebuilds: Cell::new(0),
        }
    }

    /// The URL under (`row`, `col`) on the preview half, or `None` on the
    /// source half, the divider, or empty space. Uses the exact same
    /// geometry and scroll windowing `cells` draws with, so a click always
    /// resolves the line it visibly sits on. Wired from `clickopen`'s
    /// Cmd/Ctrl+click resolution.
    pub(crate) fn link_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<String> {
        if cols == 0 || rows == 0 {
            return None;
        }
        let (_, _, right_start, right_w) = geometry(cols);
        if right_w == 0 || col < right_start {
            return None; // source side, divider column, or no room to render
        }
        let local_col = col - right_start;
        let cache = self.cache_for(cols);
        let (start, end) = window_top(cache.preview.len(), rows as usize, self.scroll_prev);
        let idx = start + row as usize;
        if idx >= end {
            return None;
        }
        crate::chatplace::cell_at_col(&cache.preview[idx], local_col)
            .and_then(|cell| cell.link.as_deref())
            .map(str::to_string)
    }

    /// Handle a winit key event: Tab flips the active side, arrows/PageUp/
    /// PageDown scroll it, `r` reloads `path` from disk, Esc asks the host to
    /// close the pane. Decoding + effects live in `mdkeys` as a pure,
    /// testable seam — mirrors `ChatPane::on_key`/`FarPane::on_key`. `cols`/
    /// `rows` let `mdkeys::reduce`'s scroll arms clamp the offset they just
    /// moved (see `clamp_scrolls`); `ctrl` — not carried on the `KeyEvent`
    /// itself, so `keys.rs` threads it from `self.mods.state()`, as it does
    /// for `InputBar::on_key` — guards Tab/`r` against a Ctrl-chord.
    pub(crate) fn on_key(
        &mut self,
        key: &KeyEvent,
        cols: u16,
        rows: u16,
        ctrl: bool,
    ) -> Option<MdAction> {
        let input = mdkeys::md_key(&key.logical_key, key.state.is_pressed(), ctrl);
        mdkeys::reduce(self, input, cols, rows)
    }

    /// Scrolls one half of the split by `delta` lines, floored at the top.
    /// This alone does *not* bound the far end — callers with real geometry
    /// (`scroll_wheel`, the key path via `mdkeys::reduce`) must follow up
    /// with `clamp_scrolls`, since a keyboard-only path (Shift+End's
    /// `i32::MIN/2` jump) can otherwise leave the stored offset around
    /// content length forever, silently swallowing every later Up tick.
    /// Convention matches `FarPane::scroll`/`ChatPane::scroll`: positive
    /// `delta` moves toward the top/start of the content.
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
    /// Shift+PageUp/Home/End — falls back to the active side. Clamps the
    /// resulting offset to `cols`×`rows` geometry afterward (see
    /// `clamp_scrolls`).
    pub(crate) fn scroll_wheel(
        &mut self,
        cols: u16,
        rows: u16,
        cursor_col: Option<u16>,
        delta: i32,
    ) {
        let (_, _, right_start, right_w) = geometry(cols);
        let side = match cursor_col {
            Some(c) if right_w > 0 && c >= right_start => Side::Preview,
            Some(_) => Side::Source,
            None => self.active,
        };
        self.scroll(side, delta);
        self.clamp_scrolls(cols, rows);
    }

    /// Re-reads `path` from disk, replacing `source` on success. On failure
    /// (missing file, permissions, non-UTF-8) the old content is left in
    /// place and an error message comes back for the caller to show as a
    /// status line — same wording as `spawn_md_pane`'s initial-read error.
    pub(crate) fn reload(&mut self) -> Result<(), String> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => {
                self.source = s;
                // The cache keys only on `cols`, not content, so a same-width
                // reload would otherwise keep serving the stale lines.
                *self.cache.get_mut() = None;
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

#[cfg(test)]
#[path = "mdpane_tests.rs"]
mod tests;
