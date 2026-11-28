//! A middle click pastes what is selected, as every X11 terminal does with
//! the primary selection: select in one pane, middle-click in another, no
//! copy step. "The selection" is the focused pane's, read before the click
//! moves focus: selecting focuses the pane, so it is the one just made. A
//! selection left in some other pane long ago is not pasted; with none it is
//! the clipboard, as right-click. The run-many-lines check applies either way.
use crate::app::CrewApp;

impl CrewApp {
    /// A right click (`middle` false) or a middle click under the cursor,
    /// after a program that owns the mouse has declined it.
    pub(crate) fn middle_or_right_paste(&mut self, middle: bool) {
        let sel = middle.then(|| self.selection_here()).flatten();
        self.focus_at_cursor();
        match sel {
            Some(text) => self.paste_text(text),
            None => self.paste(),
        }
    }

    /// The focused pane's selected text.
    pub(crate) fn selection_here(&self) -> Option<String> {
        self.pane_selection_text(self.focused)
    }
}

#[cfg(test)]
#[path = "middlepaste_tests.rs"]
mod tests;
