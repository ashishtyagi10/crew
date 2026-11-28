//! A middle click pastes what is selected, as every X11 terminal does with
//! the primary selection: select in one pane, middle-click in another, no
//! copy step. Crew keeps no selection history, so "the selection" is the
//! focused pane's, else the first pane holding one; with none it is the
//! clipboard, as right-click. The run-many-lines check applies either way.
use crate::app::CrewApp;

impl CrewApp {
    /// A right click (`middle` false) or a middle click under the cursor,
    /// after a program that owns the mouse has declined it.
    pub(crate) fn middle_or_right_paste(&mut self, middle: bool) {
        let sel = middle.then(|| self.selection_anywhere()).flatten();
        self.focus_at_cursor();
        match sel {
            Some(text) => self.paste_text(text),
            None => self.paste(),
        }
    }

    /// The selected text: the focused pane's first, then any pane's.
    pub(crate) fn selection_anywhere(&self) -> Option<String> {
        let first = std::iter::once(self.focused);
        first
            .chain(0..self.panes.len())
            .find_map(|i| self.pane_selection_text(i))
    }
}

#[cfg(test)]
#[path = "middlepaste_tests.rs"]
mod tests;
