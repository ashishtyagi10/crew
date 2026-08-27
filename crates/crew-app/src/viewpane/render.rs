//! `CardLine` → `CellView`, plus the width-keyed cache and the scroll clamp.
use std::cell::Ref;

use crew_render::CellView;

use crate::viewpane::lines;
use crate::viewpane::{ViewCache, ViewPane};

/// The live search, drawn on the pane's last row: what you are typing and how
/// many lines hold it.
///
/// Without it the viewer's `/` was typed blind — the needle existed only in
/// the pane's state, so a mistyped search looked exactly like a search with no
/// matches, and neither said which it was.
fn search_line(out: &mut Vec<CellView>, p: &ViewPane, cols: u16, rows: u16) {
    let Some(s) = &p.search else { return };
    let t = crew_theme::theme();
    let row = rows - 1;
    let count = match (s.typing, s.hits.len()) {
        (true, _) => String::new(),
        (false, 0) => "  no matches".to_string(),
        (false, n) => format!("  {n} line{}", if n == 1 { "" } else { "s" }),
    };
    let caret = if s.typing { "\u{2588}" } else { "" };
    let text = format!("/{}{caret}{count}", s.needle);
    // The row belongs to the search while it is open: clear whatever content
    // was drawn there rather than letting the two overprint.
    out.retain(|c| c.row != row);
    let fg = match (s.typing, s.hits.is_empty()) {
        (false, true) => t.bell,
        _ => crate::findhl::hit_mark(),
    };
    crate::chatwidth::place_row(0, cols, text.chars().map(|c| (c, fg)), |col, c, fg| {
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg: t.page_bg,
            ..Default::default()
        });
    });
}

impl ViewPane {
    /// Lines for `cols`, rebuilding the cache only on a width or mode change.
    pub(crate) fn lines_for(&self, cols: u16) -> Ref<'_, ViewCache> {
        let stale = self
            .cache
            .borrow()
            .as_ref()
            .is_none_or(|c| c.cols != cols || c.raw != self.raw);
        if stale {
            let (lines, marks) = lines::for_state(&self.state, self.raw, cols as usize);
            self.cache.replace(Some(ViewCache {
                cols,
                raw: self.raw,
                lines,
                marks,
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

    /// `(lines back from the bottom, total lines)` — the terminal-shaped
    /// numbers the card's scroll thumb is written in. The viewer counts rows
    /// DOWN from the top and a terminal counts them BACK from the live
    /// bottom, so this is where the two meet; getting it backwards draws a
    /// thumb that climbs as you scroll down.
    pub(crate) fn position(&self, cols: u16, rows: u16) -> (usize, usize) {
        if cols == 0 || rows == 0 {
            return (0, 0);
        }
        let total = self.lines_for(cols).lines.len();
        let visible = usize::from(rows);
        let back = total.saturating_sub(visible).saturating_sub(self.scroll);
        (back, total)
    }

    /// Rendered rows worth marking on the card's gutter: headings in a
    /// document, files and hunks in a review.
    pub(crate) fn mark_rows(&self, cols: u16) -> Vec<usize> {
        self.lines_for(cols).marks.iter().map(|m| m.row).collect()
    }

    /// Rendered rows holding a live search hit, for the card's gutter. Empty
    /// when nothing is being searched for.
    pub(crate) fn hit_rows(&self) -> Vec<usize> {
        self.search
            .as_ref()
            .map(|s| s.hits.clone())
            .unwrap_or_default()
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
        search_line(&mut out, self, cols, rows);
        out
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
