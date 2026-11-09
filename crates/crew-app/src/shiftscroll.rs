//! Shift+PageUp/PageDown and Shift+Home/End: Crew's scrollback keys.
//!
//! They page and jump a pane's history, so they are taken before any other
//! routing. On the alternate screen there is no history to move, and each one
//! goes to the program instead (`scroll.rs`): PgUp/PgDn plain, Home/End with
//! their Shift (see `altscroll` for why the two differ).
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::CrewApp;

impl CrewApp {
    /// Handle a Shift+page/end key press; false for any other key.
    pub(crate) fn shift_scroll_key(
        &mut self,
        event: &winit::event::KeyEvent,
        mstate: ModifiersState,
    ) -> bool {
        if !event.state.is_pressed() || !mstate.shift_key() {
            return false;
        }
        match &event.logical_key {
            Key::Named(NamedKey::PageUp) => self.scroll_focused_page(true),
            Key::Named(NamedKey::PageDown) => self.scroll_focused_page(false),
            Key::Named(NamedKey::Home) => self.scroll_focused_end(true),
            Key::Named(NamedKey::End) => self.scroll_focused_end(false),
            _ => return false,
        }
        true
    }
}
