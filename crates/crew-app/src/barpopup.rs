//! The input bar's palette under the mouse — the "commands" card that
//! stands over the docked bar while a slash command is typed there. Its
//! rows answer the pointer the way a pane's pop-up rows do (`chatpopup`):
//! hover moves the selection, a click picks through the same path Enter
//! takes (`InputBar::pick_menu`), the pointer wears the hand. The rows and
//! the card's place come from the same two functions `render.rs` draws
//! with, so a hit can never resolve against a row the frame did not draw.
use crate::app::{gap, CrewApp};
use crate::chrome;
use crate::layout::Rect;
use crate::suggest::MenuItem;

/// The item under surface point `(x, y)` for a `n`-row list drawn at
/// selection `sel` in the card `card` (`cw` × `ch` cells): inside the
/// frame, on a list row, mapped through the scroll the card draws with.
pub(crate) fn item_at(
    n: usize,
    sel: usize,
    card: Rect,
    cw: f32,
    ch: f32,
    x: f32,
    y: f32,
) -> Option<usize> {
    if !chrome::point_in(card, x, y) {
        return None;
    }
    let (col, row) = (((x - card.x) / cw) as u16, ((y - card.y) / ch) as u16);
    let (cols, rows) = ((card.w / cw).round() as u16, (card.h / ch).round() as u16);
    if col == 0 || col + 1 >= cols || row == 0 || row + 1 >= rows {
        return None;
    }
    let i = crate::cmdmenu::offset(n, sel) + usize::from(row - 1);
    (i < n).then_some(i)
}

impl CrewApp {
    /// The bar's palette rows: the slash commands matching the text (or
    /// its value picker, with the value you are on marked). Empty for
    /// non-slash text, where `render.rs` shows the live preview of what
    /// Enter will do instead — a row the keys never treat as a palette, so
    /// the mouse does not either.
    pub(crate) fn bar_rows(&self) -> Vec<MenuItem> {
        let mut rows = crate::cmdnote::rows(&self.input.text, &self.input.cwd);
        let cmd = self
            .input
            .text
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let current = crate::suggestvalues::current_value(cmd, &self.config);
        crate::suggest::mark_current(&mut rows, current.as_deref());
        rows
    }

    /// Where the bar's palette card is drawn this frame, with its rows —
    /// `None` when it is not (bar unfocused, nothing to list, no renderer).
    fn bar_popup_geometry(&self) -> Option<(Rect, Vec<MenuItem>)> {
        if !self.input.focused {
            return None;
        }
        let (cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let rows = self.bar_rows();
        if rows.is_empty() {
            return None;
        }
        let ih = chrome::bottom_chrome_h(sh, ch, gap());
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), gap(), ih);
        let ib = chrome::inputbar_rect(content, sh, ch, gap());
        let ic = (ib.w / cw).floor() as u16;
        let (cols, card_rows) = crate::cmdmenu::popup_size(&rows, ic);
        let h = f32::from(card_rows) * ch;
        let card = Rect {
            x: ib.x,
            y: (ib.y - h - gap()).max(0.0),
            w: f32::from(cols) * cw,
            h,
        };
        Some((card, rows))
    }

    /// The palette row under the cursor, with the rows it is one of.
    fn bar_popup_item_at_cursor(&self) -> Option<(usize, Vec<MenuItem>)> {
        let (card, rows) = self.bar_popup_geometry()?;
        let (cw, ch, ..) = self.frame_geometry()?;
        let (x, y) = self.cursor;
        let i = item_at(rows.len(), self.input.menu_sel, card, cw, ch, x, y)?;
        Some((i, rows))
    }

    /// Hover: the row under the pointer becomes the selection (a section
    /// title is not a choice). `true` when it moved — the repaint signal.
    pub(crate) fn bar_popup_hover_sync(&mut self) -> bool {
        let Some((i, rows)) = self.bar_popup_item_at_cursor() else {
            return false;
        };
        if rows[i].header || !crate::cmdnote::selectable(&rows) || self.input.menu_sel == i {
            return false;
        }
        self.input.menu_sel = i;
        true
    }

    /// Click: pick the row under the cursor as Enter would. `true` when a
    /// row took the click. A picked line is submitted exactly as a typed
    /// one; a `/quit` picked this way closes the canvas the graceful way.
    pub(crate) fn bar_popup_click(&mut self) -> bool {
        let Some((i, rows)) = self.bar_popup_item_at_cursor() else {
            return false;
        };
        if rows[i].header || !crate::cmdnote::selectable(&rows) {
            return true;
        }
        self.input.menu_sel = i;
        if let Some(line) = self.input.pick_menu(&rows) {
            crate::themepeek::accept();
            if self.submit_input(line) {
                self.closing = true;
            }
            crate::history::save(&self.input.history);
        }
        true
    }

    /// Whether a bar palette row is under the pointer — the hand.
    pub(crate) fn bar_popup_hover_on(&self) -> bool {
        self.bar_popup_item_at_cursor().is_some()
    }
}

#[cfg(test)]
#[path = "barpopup_tests.rs"]
mod tests;
