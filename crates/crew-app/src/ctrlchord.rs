//! Where crew's Cmd chords land on Windows and Linux (`keychord::stand_in`):
//! whether a plain Ctrl key here belongs to a terminal or a line being typed,
//! and the modifier the grid's arrows take. Windows keeps Win+Arrow for
//! snapping windows, so off a Mac the arrows walk the grid with Alt, as
//! Windows Terminal walks its panes.
use winit::keyboard::ModifiersState;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Whether a plain Ctrl key here goes to a program or a typed line: the
    /// input bar, a terminal, or a pane with a line to type into.
    pub(crate) fn typing_here(&self) -> bool {
        let typed = |p: &crate::pane::Pane| {
            matches!(
                p.content,
                PaneContent::Terminal(_)
                    | PaneContent::Chat(_)
                    | PaneContent::Todo(_)
                    | PaneContent::Settings(_)
                    | PaneContent::Far(_)
            )
        };
        self.input.focused || self.panes.get(self.focused).is_some_and(typed)
    }

    /// The Cmd chord a key press makes here ([`crate::keychord::cmd_chord`]).
    pub(crate) fn chord_here(
        &self,
        event: &winit::event::KeyEvent,
        mods: ModifiersState,
    ) -> Option<String> {
        crate::keychord::cmd_chord(event, mods, self.typing_here())
    }
}

/// Whether the grid's arrows are held: Cmd, and off a Mac Alt (not AltGr).
pub(crate) fn grid_held(mods: ModifiersState, mac: bool) -> bool {
    mods.super_key() || (!mac && mods.alt_key() && !mods.control_key())
}
