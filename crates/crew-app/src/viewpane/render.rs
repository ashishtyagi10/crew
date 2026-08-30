//! `CardLine` → `CellView`, plus the width-keyed cache and the scroll clamp.
use std::cell::Ref;

use crew_render::CellView;

use crate::viewpane::lines;
use crate::viewpane::sticky;
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
        // The blame column is part of the layout, not a decoration on top of
        // it: the text is wrapped at what is left after the column, so a
        // rendering made without one cannot have it added later.
        let blame_w = self
            .blame
            .lines()
            .and_then(|_| crate::viewpane::blame::width_for(cols as usize))
            .unwrap_or(0);
        let theme = crew_theme::current_id();
        let stale = self.cache.borrow().as_ref().is_none_or(|c| {
            c.cols != cols
                || c.raw != self.raw
                || c.blame_w != blame_w
                || c.theme != theme
                || c.invisibles != crate::invisibles::on()
                || c.split != self.split
        });
        if stale {
            let text_cols = (cols as usize).saturating_sub(blame_w);
            let invisibles = crate::invisibles::on();
            let (mut lines, marks, pictures) =
                lines::for_state(&self.state, self.raw, text_cols, invisibles, self.split);
            if let Some(b) = self.blame.lines().filter(|_| blame_w > 0) {
                let labels = crate::viewpane::blame::labels(b, blame_w);
                crate::viewpane::blamegutter::apply(&mut lines, &labels, blame_w);
            }
            self.cache.replace(Some(ViewCache {
                cols,
                raw: self.raw,
                lines,
                marks,
                pictures,
                blame_w,
                invisibles,
                split: self.split,
                theme,
            }));
        }
        Ref::map(self.cache.borrow(), |c| c.as_ref().expect("just filled"))
    }

    /// The URL the caret is inside, if it is inside one.
    ///
    /// A link's target is invisible in a render — that is what rendering a
    /// link MEANS — so the window says it while the cursor is in one. The
    /// cells already carry it: `chatmd` tags every character of a link span
    /// with the URL so a click can recover it without re-parsing.
    pub(crate) fn caret_link(&self, cols: u16) -> Option<String> {
        let c = self.caret?;
        let cache = self.lines_for(cols);
        let line = cache.lines.get(c.row)?;
        let mut col = 0u16;
        for cell in line {
            let w = crate::chatwidth::char_w(cell.c) as u16;
            if w == 0 {
                continue;
            }
            if col == c.col {
                return cell.link.as_deref().map(str::to_string);
            }
            col += w;
        }
        None
    }

    /// Put the caret where a click landed, in rendered rows and columns.
    pub(crate) fn click_caret(&mut self, row: usize, col: u16, cols: u16, rows: u16) {
        if self.caret.is_none() {
            return;
        }
        let to = {
            let cache = self.lines_for(cols);
            super::caret::at_cell(&cache.lines, self.scroll + row, col)
        };
        if to.is_some() {
            self.clear_selection();
            self.set_caret(to, cols);
            // Moving the caret by hand ends the run of typing: what is typed
            // next is a separate thing you did.
            self.history.breaks();
            self.scroll_to_caret(rows);
        }
    }

    /// Move the caret one step and scroll to keep it on screen. `None` when
    /// this document is not being edited.
    pub(crate) fn move_caret(&mut self, dir: super::caret::Step, cols: u16, rows: u16) {
        let Some(here) = self.caret else { return };
        let moved = {
            let cache = self.lines_for(cols);
            super::caret::step(&cache.lines, here, dir)
        };
        self.set_caret(Some(moved), cols);
        self.history.breaks();
        self.scroll_to_caret(rows);
    }

    /// Put the caret on the document's first editable place, if it has one.
    pub(crate) fn start_editing(&mut self, cols: u16) {
        if self.caret.is_some() {
            return;
        }
        let at = {
            let cache = self.lines_for(cols);
            super::caret::first(&cache.lines)
        };
        self.set_caret(at, cols);
    }

    /// Move the caret and stamp the byte it is now on.
    fn set_caret(&mut self, to: Option<super::caret::Caret>, cols: u16) {
        let at = to.and_then(|c| {
            let cache = self.lines_for(cols);
            super::caret::offset_at(&cache.lines, c)
        });
        self.caret = to;
        self.caret_at = at;
    }

    /// Find the caret again after the document was laid out at a new width.
    /// The byte is what the caret IS; the row and column are only where this
    /// layout happens to put it.
    pub(crate) fn relayout_caret(&mut self, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let found = {
            let cache = self.lines_for(cols);
            super::caret::find(&cache.lines, at)
        };
        if let Some(c) = found {
            self.caret = Some(c);
            self.scroll_to_caret(rows);
        }
    }

    /// Scroll the least that puts the caret back in view. A caret you cannot
    /// see is a caret you cannot type at.
    fn scroll_to_caret(&mut self, rows: u16) {
        let Some(c) = self.caret else { return };
        let rows = usize::from(rows).max(1);
        // One row of slack at each edge, so the line being typed on is never
        // the very first or very last thing on screen.
        let margin = usize::from(rows > 4);
        if c.row < self.scroll + margin {
            self.scroll = c.row.saturating_sub(margin);
        } else if c.row + margin >= self.scroll + rows {
            self.scroll = c.row + margin + 1 - rows;
        }
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

    /// The decoded picture this pane is holding, if it is holding one.
    pub(crate) fn image(&self) -> Option<&super::bitmap::Bitmap> {
        match &self.state {
            crate::viewpane::LoadState::Ready { loaded, .. } => loaded.image.as_ref(),
            _ => None,
        }
    }

    /// Cells *and* the paint under them. Every rung but one draws nothing on
    /// the paint layer; the image rung draws almost nothing on the cell one —
    /// a banner naming the file, and the picture itself in the rows below it.
    pub(crate) fn art(
        &self,
        cols: u16,
        rows: u16,
        aspect: f32,
    ) -> (Vec<CellView>, Vec<crew_render::Paint>) {
        let cells = self.cells(cols, rows);
        let Some(bm) = self.image() else {
            // Not a picture FILE, but the document may still name some.
            return (cells, self.named_pictures(cols, rows, aspect));
        };
        let paint = super::bitmap::paint(bm, cols, rows.saturating_sub(1), aspect)
            .into_iter()
            .map(|p| p.shifted(0.0, 1.0))
            .collect();
        (cells, paint)
    }

    /// The pictures this document NAMES, drawn into the rows the layout
    /// reserved for them — clipped to the pane, because a document scrolls and
    /// paint is not clipped by anything else.
    fn named_pictures(&self, cols: u16, rows: u16, aspect: f32) -> Vec<crew_render::Paint> {
        let cache = self.lines_for(cols);
        if cache.pictures.is_empty() {
            return Vec::new();
        }
        let top = self.scroll;
        // Rows a picture must not enter: the sticky heading band owns the
        // first, a live search owns the last. Both are chrome the document
        // scrolls UNDER, and paint is drawn over a cell's background — so
        // without this a picture scrolled halfway off the top is drawn over
        // the band naming the section it is in.
        let y0 = f32::from(u16::from(sticky::label_for(&cache.marks, top).is_some()));
        let y1 = f32::from(rows) - f32::from(u16::from(self.search.is_some()));
        let mut out = Vec::new();
        for p in &cache.pictures {
            // Wholly above or below the window: not merely invisible, but not
            // worth resolving a path or rasterizing for.
            if p.row + p.rows <= top || p.row >= top + usize::from(rows) {
                continue;
            }
            let Some(path) = crate::imgcache::resolve(&p.src, &self.path) else {
                continue;
            };
            let Some(bm) = crate::imgcache::get(&path) else {
                continue;
            };
            let y = p.row as f32 - top as f32;
            out.extend(super::bitmap::paint_at(
                &bm,
                1.0,
                y,
                f32::from(cols).max(2.0) - 2.0,
                p.rows as f32,
                aspect,
                (0.0, y0, f32::from(cols), y1),
            ));
        }
        out
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
