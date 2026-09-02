//! Moving the viewer's caret: where a click puts it, how a key moves it, and
//! keeping it in view when it does.
//!
//! Split from [`super::render`] for the line cap. Named for the motion rather
//! than the caret — [`super::caret`] already exists and holds the caret's own
//! model.
use crate::viewpane::ViewPane;

impl ViewPane {
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
            super::caretfind::at_cell(&cache.lines, self.scroll + row, col)
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
    ///
    /// Except after a re-read of a file that got shorter: a byte past the new
    /// end is not a place in this text, and a caret drawn at the end while
    /// still holding it would splice the next keystroke into nothing. It is
    /// clamped to the end; a byte that exists is kept exactly.
    pub(crate) fn relayout_caret(&mut self, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let len = self.source_str().map_or(at, |s| s.len() as u32);
        let at = at.min(len);
        self.caret_at = Some(at);
        let found = {
            let cache = self.lines_for(cols);
            super::caretfind::find(&cache.lines, at)
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
}
