//! The app side of a pop-up under the mouse: the press that picks a row,
//! the hover that moves the selection, and the hand the pointer wears over
//! a row. Mirrors `chatplanclick` for the plan buttons; the geometry is
//! `ChatPane::popup_item_at`, so a hit can never resolve against a row the
//! frame did not draw.
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// The focused chat pane's cell under the cursor, if a pop-up can be
    /// there: a pop-up is only ever drawn over the FOCUSED pane, and never
    /// while the input bar has the keys (`render.rs` draws under the same
    /// two conditions).
    fn popup_cell_at_cursor(&self) -> Option<(usize, u16, u16)> {
        let (i, row, col) = self.chat_cell_at_cursor()?;
        (i == self.focused && !self.input.focused).then_some((i, row, col))
    }

    /// Left press: a click on a pop-up row picks it. `true` when it did —
    /// the caller answers the press there and never lets it reach the pane
    /// underneath (a pop-up is an overlay, like a toast).
    pub(crate) fn popup_press_at_cursor(&mut self) -> bool {
        let Some((i, row, col)) = self.popup_cell_at_cursor() else {
            return false;
        };
        let grid = self.panes[i].grid;
        let cwd = self.cwd.clone();
        let PaneContent::Chat(chat) = &mut self.panes[i].content else {
            return false;
        };
        let Some(action) = chat.popup_click_at(grid.cols, grid.rows, row, col, &cwd) else {
            return false;
        };
        if let Some(action) = action {
            self.apply_chat_action(action, i);
        }
        true
    }

    /// Re-publish the pop-up row under the pointer. `true` when the
    /// selection moved — the repaint signal (`pointer::pointer_sync`).
    pub(crate) fn popup_hover_sync(&mut self) -> bool {
        let Some((i, row, col)) = self.popup_cell_at_cursor() else {
            return false;
        };
        let grid = self.panes[i].grid;
        let PaneContent::Chat(chat) = &mut self.panes[i].content else {
            return false;
        };
        chat.popup_hover_at(grid.cols, grid.rows, Some((row, col)))
    }

    /// Whether pane `i` has a pop-up row under the pointer — what the pointer
    /// shape asks (the same hand a link and a plan button get).
    pub(crate) fn popup_hover_on(&self, i: usize) -> bool {
        let Some((at, row, col)) = self.popup_cell_at_cursor() else {
            return false;
        };
        let grid = self.panes[i].grid;
        match &self.panes[i].content {
            PaneContent::Chat(c) if at == i => {
                c.popup_item_at(grid.cols, grid.rows, row, col).is_some()
            }
            _ => false,
        }
    }
}
