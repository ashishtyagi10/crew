//! The pointer shape a terminal program asks for with OSC 22.
//!
//! lazygit, yazi, helix and textual apps draw their own buttons and ask for a
//! hand over them; crew used to answer every cell with the I-beam. The shape
//! is the lowest-priority answer in [`crate::pointer`]: links, gutters,
//! handles and controls are crew's own and always win, and a selection drag
//! is crew's gesture, so it keeps the text cursor.
use winit::window::CursorIcon;

use crate::app::CrewApp;
use crate::pane::PaneContent;

/// A CSS cursor name (OSC 22's vocabulary, winit's too) as an icon. `None`
/// for a name winit does not know, so crew keeps its own default; kitty's
/// comma-separated fallback list takes the first name that is known.
pub(crate) fn css_icon(name: &str) -> Option<CursorIcon> {
    name.split(',')
        .map(str::trim)
        .find_map(|n| n.parse::<CursorIcon>().ok())
}

impl CrewApp {
    /// The icon pane `i`'s program asked for, when it is in control of the
    /// pointer: a terminal pane, a shape it set, and no selection drag.
    pub(crate) fn program_pointer(&self, i: usize) -> Option<CursorIcon> {
        if self.drag.is_some_and(|d| d.moved) {
            return None;
        }
        let PaneContent::Terminal(t) = &self.panes.get(i)?.content else {
            return None;
        };
        css_icon(t.pty.pointer_shape()?)
    }
}

#[cfg(test)]
#[path = "pointerprog_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pointerprog_app_tests.rs"]
mod app_tests;
