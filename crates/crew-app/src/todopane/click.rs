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
            TodoClick::PickTag(i) => t.pick_tag(i),
        }
        self.focused = i;
        self.input.focused = false;
        true
    }
}

impl CrewApp {
    /// The todo pane and tag row under the pointer, if the pointer is on
    /// an open tag pop-up.
    fn todo_tag_at_cursor(&self) -> Option<(usize, usize)> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Todo(t) = &self.panes[i].content else {
            return None;
        };
        let tag = super::measure::tag_under(t, row as u16, col as u16, grid.cols, grid.rows)?;
        Some((i, tag))
    }

    /// Hover: the tag row under the pointer becomes the selection, as it
    /// does on every other pop-up. `true` when it moved — the repaint signal.
    pub(crate) fn todo_hover_sync(&mut self) -> bool {
        let Some((i, tag)) = self.todo_tag_at_cursor() else {
            return false;
        };
        let crate::pane::PaneContent::Todo(t) = &mut self.panes[i].content else {
            return false;
        };
        let Some(m) = &mut t.tagmenu else {
            return false;
        };
        let moved = m.sel != tag;
        m.sel = tag;
        moved
    }

    /// Whether pane `i` has a tag row under the pointer — the hand.
    pub(crate) fn todo_hover_on(&self, i: usize) -> bool {
        self.todo_tag_at_cursor().is_some_and(|(at, _)| at == i)
    }
}

impl super::TodoPane {
    /// Accept the pop-up's `i`th tag into the composer, as Enter does on
    /// the selected one; the pop-up closes either way.
    pub(crate) fn pick_tag(&mut self, i: usize) {
        if let Some((sigil, tag)) = self
            .tagmenu
            .as_ref()
            .and_then(|m| Some((m.sigil, m.matches.get(i)?.clone())))
        {
            self.input = super::tagmenu::accept(&self.input, sigil, &tag);
            self.cursor = self.input.chars().count();
        }
        self.tagmenu = None;
    }
}

#[cfg(test)]
#[path = "tagclick_tests.rs"]
mod tagclick_tests;
