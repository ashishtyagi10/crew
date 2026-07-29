//! `CardLine` → `CellView`, plus the width-keyed cache and the scroll clamp.
use std::cell::Ref;

use crew_render::CellView;

use crate::viewpane::lines;
use crate::viewpane::{LoadState, ViewCache, ViewPane};

impl ViewPane {
    /// Lines for `cols`, rebuilding the cache only on a width or mode change.
    pub(crate) fn lines_for(&self, cols: u16) -> Ref<'_, ViewCache> {
        let stale = self
            .cache
            .borrow()
            .as_ref()
            .is_none_or(|c| c.cols != cols || c.raw != self.raw);
        if stale {
            self.cache.replace(Some(ViewCache {
                cols,
                raw: self.raw,
                lines: lines::for_state(&self.state, self.raw, cols as usize),
            }));
        }
        Ref::map(self.cache.borrow(), |c| c.as_ref().expect("just filled"))
    }

    /// Cap the stored offset to the last full page — not merely the drawn
    /// view, or a big jump leaves later scroll ticks dead.
    pub(crate) fn clamp_scroll(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        let len = self.lines_for(cols).lines.len();
        self.scroll = self.scroll.min(len.saturating_sub(rows as usize));
    }

    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        let page_bg = crew_theme::theme().page_bg;
        let cache = self.lines_for(cols);
        let top = self.scroll.min(cache.lines.len().saturating_sub(1));
        let mut out = Vec::new();
        for (r, line) in cache.lines.iter().skip(top).take(rows as usize).enumerate() {
            let mut col = 0u16;
            for cell in line {
                if col >= cols {
                    break;
                }
                out.push(CellView {
                    col,
                    row: r as u16,
                    c: cell.c,
                    fg: cell.fg,
                    bg: cell.bg.unwrap_or(page_bg),
                    bold: cell.bold,
                    italic: cell.italic,
                });
                col += crate::chatwidth::char_w(cell.c).max(1) as u16;
            }
        }
        out
    }

    /// True while the worker has not landed — read by `poll.rs`'s animation
    /// gate so the skeleton animates and, crucially, stops.
    pub(crate) fn animating(&self) -> bool {
        matches!(self.state, LoadState::Loading { .. })
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
