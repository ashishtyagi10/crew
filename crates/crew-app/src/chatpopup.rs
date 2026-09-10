//! The composer pop-up as ONE thing the pane has: which of the six is open,
//! how it is drawn, and which of its rows the mouse is on.
//!
//! `render.rs` used to hold five copies of "if this state is open, build
//! its card and place it", one per pop-up, in an order that had to match
//! the key chain in `chattype.rs` by hand. The mouse needs the same answer
//! — the card's rows are where a hover lands and a click picks — so the
//! precedence lives here once: the key prompt (it swallows every key), then
//! find, history, the attach picker and the palette.
use crate::chat::ChatPane;
use crate::popupplace::Popup;
use crate::suggest::MenuItem;

/// The attach picker's rows: `@token` and what it is.
fn attach_items(m: &crate::chatmention::MentionState) -> Vec<MenuItem> {
    m.matches
        .iter()
        .map(|e| MenuItem {
            label: format!("@{}", e.token()),
            desc: e.desc(),
            ..Default::default()
        })
        .collect()
}

impl ChatPane {
    /// The pop-up open on this pane, drawn for a pane `cols` wide. `None`
    /// when nothing is open (or the open list is empty — `after_edit` clears
    /// an empty palette, so that arm is a guard, not a state).
    pub(crate) fn popup(&self, cols: u16) -> Option<Popup> {
        if let Some(e) = &self.keyentry {
            return Some(e.card(cols));
        }
        if let Some(f) = &self.find {
            return Some(crate::chatfind::card(f, &self.visible_messages(), cols));
        }
        if let Some(h) = &self.histsearch {
            return Some(crate::chathistsearch::card(h, cols));
        }
        if let Some(m) = self.mention.as_ref().filter(|m| !m.matches.is_empty()) {
            let items = attach_items(m);
            return Some(crate::cmdmenu::popup("attach", &items, m.sel, cols));
        }
        let p = self.palette.as_ref().filter(|p| !p.items.is_empty())?;
        let title = crate::render::palette_card_title(p.kind);
        Some(crate::cmdmenu::popup(title, &p.items, p.sel, cols))
    }

    /// The open LIST pop-up's `(rows, selection)`; `None` for the key prompt
    /// (a field, not a list) and when nothing is open. Same precedence as
    /// [`Self::popup`], so a row the mouse resolves is a row that was drawn.
    fn popup_list(&self) -> Option<(usize, usize)> {
        if self.keyentry.is_some() {
            return None;
        }
        if let Some(f) = &self.find {
            return Some((f.matches.len().max(1), f.sel));
        }
        if let Some(h) = &self.histsearch {
            return Some((h.matches.len().max(1), h.sel));
        }
        if let Some(m) = self.mention.as_ref().filter(|m| !m.matches.is_empty()) {
            return Some((m.matches.len(), m.sel));
        }
        let p = self.palette.as_ref().filter(|p| !p.items.is_empty())?;
        Some((p.items.len(), p.sel))
    }

    /// The list item under pane cell `(row, col)` while a list pop-up is
    /// open: inside the frame, on a list row, mapped through the scroll the
    /// card draws with. `None` on the frame, off the card, or with no list.
    pub(crate) fn popup_item_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<usize> {
        let (n, sel) = self.popup_list()?;
        let p = self.popup(cols)?;
        // Where `popupplace::above_composer` stands the card, in cells.
        let g = crate::chatplace::grants(self, cols, rows);
        let top = rows.saturating_sub(g.bottom).saturating_sub(p.rows);
        let inside = col > 0 && col + 1 < p.cols && row > top && row + 1 < top + p.rows;
        if !inside {
            return None;
        }
        let i = crate::cmdmenu::offset(n, sel) + usize::from(row - top - 1);
        (i < n).then_some(i)
    }

    /// Make item `i` the selection, as an arrow would — a section title is
    /// not a choice and is skipped. `true` when the selection moved.
    fn popup_select(&mut self, i: usize) -> bool {
        let sel = if let Some(f) = &mut self.find {
            &mut f.sel
        } else if let Some(h) = &mut self.histsearch {
            &mut h.sel
        } else if let Some(m) = &mut self.mention {
            &mut m.sel
        } else if let Some(p) = &mut self.palette {
            if p.items.get(i).is_none_or(|it| it.header) {
                return false;
            }
            // A pointed-at row is a chosen row: the untouched-Enter rule
            // that submits a bare `/model` no longer applies.
            p.touched = true;
            &mut p.sel
        } else {
            return false;
        };
        let moved = *sel != i;
        *sel = i;
        moved
    }

    /// Hover: the row under the pointer becomes the selection, as it does in
    /// every menu. `true` when it moved — the repaint signal.
    pub(crate) fn popup_hover_at(&mut self, cols: u16, rows: u16, at: Option<(u16, u16)>) -> bool {
        match at.and_then(|(r, c)| self.popup_item_at(cols, rows, r, c)) {
            Some(i) => self.popup_select(i),
            None => false,
        }
    }

    /// Click on `(row, col)`: pick the row there — select it, then take the
    /// same Enter the keyboard would, so a mouse pick and a key pick are one
    /// path. `None` when no pop-up row was under the click; otherwise the
    /// action the Enter produced (usually none).
    pub(crate) fn popup_click_at(
        &mut self,
        cols: u16,
        rows: u16,
        row: u16,
        col: u16,
        cwd: &std::path::Path,
    ) -> Option<Option<crate::chatkeys::ChatAction>> {
        let i = self.popup_item_at(cols, rows, row, col)?;
        self.popup_select(i);
        Some(self.on_input(crate::chatkeys::ChatInput::Enter, cwd))
    }
}

#[cfg(test)]
#[path = "chatpopup_tests.rs"]
mod tests;
