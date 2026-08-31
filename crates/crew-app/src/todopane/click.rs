//! Clicking a todo: the `CrewApp` end of it, which turns a pointer position
//! into the row it landed on and the action that row offers.
//!
//! Split out of [`super`] for the line cap. It hangs off `CrewApp` rather than
//! `TodoPane` because the click arrives before anything knows which pane it
//! was for.
use super::{render, TodoClick};
use crate::app::CrewApp;

impl CrewApp {
    /// A left click inside a todo pane acts where it lands: the checkbox
    /// toggles, the `✗` deletes, a row selects, the composer refocuses —
    /// and the pane takes focus. Empty regions return `false` and fall
    /// through to the normal focus/drag path.
    pub(crate) fn todo_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        if !matches!(self.panes[i].content, crate::pane::PaneContent::Todo(_)) {
            return false;
        }
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Todo(t) = &mut self.panes[i].content else {
            return false;
        };
        let Some(click) = render::click_at(t, row as u16, col as u16, grid.cols, grid.rows) else {
            return false;
        };
        match click {
            TodoClick::Toggle(d) => t.toggle_done_at(d),
            TodoClick::Delete(d) => t.delete_at(d),
            TodoClick::Select(d) => t.sel = Some(d),
            TodoClick::Composer => t.sel = None,
            TodoClick::ShowDone => t.set_show_done(!t.show_done),
        }
        self.focused = i;
        self.input.focused = false;
        true
    }
}
