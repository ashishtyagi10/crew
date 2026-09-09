//! `CardLine` → `CellView`, plus the width-keyed cache and the scroll clamp.
use std::cell::Ref;

use crew_render::CellView;

use crate::viewpane::lines;
use crate::viewpane::searchline::search_line;
use crate::viewpane::sticky;
use crate::viewpane::{lspdeco, lspgutter};
use crate::viewpane::{ViewCache, ViewPane};

impl ViewPane {
    /// Lines for `cols`, rebuilding the cache only on a width or mode change.
    pub(crate) fn lines_for(&self, cols: u16) -> Ref<'_, ViewCache> {
        // The blame column is part of the layout, not a decoration on top of
        // it: the text is wrapped at what is left after the column, so a
        // rendering made without one cannot have it added later.
        let blame_w = self
            .blame
            .lines()
            .and_then(|_| crate::viewpane::blame::width_for(cols as usize))
            .unwrap_or(0);
        // …and so is the diagnostics margin, for the same reason.
        let lsp_w = self.lsp.diags().map_or(0, |_| lspgutter::WIDTH);
        let theme = crew_theme::current_id();
        let stale = self.cache.borrow().as_ref().is_none_or(|c| {
            c.cols != cols
                || c.raw != self.raw
                || c.blame_w != blame_w
                || c.lsp_w != lsp_w
                || c.theme != theme
                || c.invisibles != crate::invisibles::on()
                || c.split != self.split
        });
        if stale {
            let text_cols = (cols as usize).saturating_sub(blame_w + lsp_w);
            let invisibles = crate::invisibles::on();
            let (mut lines, marks, pictures) =
                lines::for_state(&self.state, self.raw, text_cols, invisibles, self.split);
            if let Some(b) = self.blame.lines().filter(|_| blame_w > 0) {
                let labels = crate::viewpane::blame::labels(b, blame_w);
                crate::viewpane::blamegutter::apply(&mut lines, &labels, blame_w);
            }
            if let Some(d) = self.lsp.diags() {
                lspgutter::apply(&mut lines, &lspgutter::marks(d), blame_w);
            }
            self.cache.replace(Some(ViewCache {
                cols,
                raw: self.raw,
                lines,
                marks,
                pictures,
                blame_w,
                lsp_w,
                invisibles,
                split: self.split,
                theme,
            }));
        }
        Ref::map(self.cache.borrow(), |c| c.as_ref().expect("just filled"))
    }

    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        let page_bg = crew_theme::theme().page_bg;
        // A zero-byte file drew nothing at all: a titled empty box, with no
        // way to tell it from a load still in flight.
        if let crate::viewpane::LoadState::Ready { loaded, .. } = &self.state {
            if loaded.text.is_empty() && loaded.image.is_none() {
                return crate::toosmall::row("(empty file)", cols);
            }
        }
        let cache = self.lines_for(cols);
        let top = self.scroll.min(cache.lines.len().saturating_sub(1));
        let mut out = Vec::new();
        for (r, line) in cache.lines.iter().skip(top).take(rows as usize).enumerate() {
            let row = r as u16;
            // `place_row` guards on the glyph's full display width before
            // placing it (`x + w > max_col`), not merely on where it starts —
            // a char-count guard lets a 2-wide glyph land one column before
            // the edge and overrun it. Reuse the same guard the rest of the
            // chat views use rather than mirror it.
            crate::chatwidth::place_row(
                0,
                cols,
                line.iter().map(|cell| {
                    (
                        cell.c,
                        (cell.fg, cell.bg.unwrap_or(page_bg), cell.bold, cell.italic),
                    )
                }),
                |col, c, (fg, bg, bold, italic)| {
                    out.push(CellView {
                        col,
                        row,
                        c,
                        fg,
                        bg,
                        bold,
                        italic,
                        ..Default::default()
                    });
                },
            );
        }
        // What is selected, washed the way every other selection in crew is:
        // a range of BYTES, so the wash follows the text through a re-wrap
        // instead of being a rectangle of the screen.
        if let Some((lo, hi)) = super::select::range(self) {
            let bg = crew_theme::theme().find_hl_bg;
            let mut col = 0u16;
            for (r, line) in cache.lines.iter().skip(top).take(rows as usize).enumerate() {
                col = 0;
                for cell in line {
                    let w = crate::chatwidth::char_w(cell.c) as u16;
                    if w == 0 {
                        continue;
                    }
                    if cell.src.is_some_and(|s| s >= lo && s < hi) {
                        if let Some(c) = out.iter_mut().find(|c| c.row == r as u16 && c.col == col)
                        {
                            c.bg = bg;
                        }
                    }
                    col += w;
                }
            }
            let _ = col;
        }
        // What the language server flagged, underlined where it said.
        if let (Some(d), crate::viewpane::LoadState::Ready { loaded, .. }) =
            (self.lsp.diags(), &self.state)
        {
            let spans = lspdeco::spans(&loaded.text, d, crate::invisibles::on());
            let at = cache.blame_w + cache.lsp_w;
            lspdeco::apply(&mut out, &cache.lines, top, rows as usize, at, &spans);
        }
        // The caret, on the cell it is standing on. A beam rather than a
        // block: the character under it is the document, and a block would
        // hide the very letter you are about to type beside.
        if let Some(caret) = self.caret {
            let row = caret.row.checked_sub(top);
            if let Some(row) = row.filter(|r| *r < usize::from(rows)) {
                let mark = crew_theme::deco::CursorMark {
                    shape: crew_theme::deco::CursorShape::Beam,
                    color: crate::palette::accent(),
                };
                if let Some(cell) = out
                    .iter_mut()
                    .find(|c| c.row == row as u16 && c.col == caret.col)
                {
                    cell.cursor = mark;
                }
            }
        }
        // The heading this row is underneath, kept where the address is
        // (see `sticky`). Before the search line, which owns the LAST row and
        // must win over nothing here.
        if let Some(label) = sticky::label_for(&cache.marks, top) {
            sticky::draw(&mut out, &label, cols);
        }
        search_line(&mut out, self, cols, rows);
        out
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
