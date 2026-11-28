//! Keys for a terminal pane's find bar: Cmd+F opens it, and while it is open
//! it owns the keyboard (the same way hint mode does) — typed letters build
//! the query, Enter / Shift+Enter step, Esc closes — so nothing typed in it
//! ever reaches the program in the pane.
//!
//! Closing leaves the view where it is, as Ghostty does: after searching you
//! usually want to read around the match you stopped on, and snapping back to
//! the bottom would throw that place away (End / a keystroke in the shell
//! already does).
use crate::app::CrewApp;
use crate::pane::{PaneContent, TermPane};
use crate::termfind::{FindKey, TermFind};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Keys that are never the query's: Ctrl+Tab walks panes, and Shift+PageUp /
/// PageDown / Home / End scroll the buffer being searched.
fn passes_through(key: &Key, mods: ModifiersState) -> bool {
    match key {
        Key::Named(NamedKey::Tab) => mods.control_key(),
        Key::Named(NamedKey::PageUp | NamedKey::PageDown | NamedKey::Home | NamedKey::End) => {
            mods.shift_key()
        }
        _ => false,
    }
}

impl CrewApp {
    /// The focused pane's terminal, when the find bar is open on it and the
    /// keys are not somewhere else (the input bar, the nav list).
    fn find_term(&mut self) -> Option<&mut TermPane> {
        if self.input.focused || self.nav_has_keys() {
            return None;
        }
        match &mut self.panes.get_mut(self.focused)?.content {
            PaneContent::Terminal(t) if t.find.is_some() => Some(t),
            _ => None,
        }
    }

    /// Cmd+F on a terminal opens (or keeps) its find bar and takes the keys.
    /// False when the focused pane is not a terminal.
    pub(crate) fn term_find_open(&mut self) -> bool {
        let Some(pane) = self.panes.get_mut(self.focused) else {
            return false;
        };
        let PaneContent::Terminal(t) = &mut pane.content else {
            return false;
        };
        t.find.get_or_insert_with(TermFind::default);
        self.input.focused = false;
        true
    }

    /// While the bar is open, every key that is not a crew chord is the
    /// bar's. True when the key was consumed.
    pub(crate) fn term_find_key(
        &mut self,
        event: &winit::event::KeyEvent,
        mods: ModifiersState,
    ) -> bool {
        self.term_find_event(&event.logical_key, event.state.is_pressed(), mods)
    }

    /// [`CrewApp::term_find_key`] over the parts of a key press, which tests
    /// can build (a winit `KeyEvent` they cannot).
    pub(crate) fn term_find_event(
        &mut self,
        key: &Key,
        pressed: bool,
        mods: ModifiersState,
    ) -> bool {
        self.close_find_off_focus();
        if self.find_term().is_none() {
            return false;
        }
        if crate::ctrlchord::cmd_held(mods, cfg!(target_os = "macos")) {
            return false;
        }
        // Scrolling and pane-walking keys stay the app's (Ghostty scrolls
        // during a search): without this they would fall to `Other` and be
        // eaten, and the bar hooks in before `shift_scroll_key`.
        if passes_through(key, mods) {
            return false;
        }
        if !pressed {
            return true;
        }
        let key = match key {
            Key::Named(NamedKey::Escape) => FindKey::Close,
            Key::Named(NamedKey::Enter) if mods.shift_key() => FindKey::Newer,
            Key::Named(NamedKey::Enter | NamedKey::ArrowUp) => FindKey::Older,
            Key::Named(NamedKey::ArrowDown) => FindKey::Newer,
            Key::Named(NamedKey::Backspace) => FindKey::Backspace,
            Key::Named(NamedKey::Space) => FindKey::Char(' '),
            Key::Character(s) if !mods.control_key() => {
                let mut cs = s.chars().filter(|c| !c.is_control());
                match (cs.next(), cs.next()) {
                    (Some(c), None) => FindKey::Char(c),
                    _ => FindKey::Other,
                }
            }
            _ => FindKey::Other,
        };
        self.term_find_input(key)
    }

    /// Apply one bar key to the focused terminal's search; false when no bar
    /// is open there (the key belongs to the pane).
    pub(crate) fn term_find_input(&mut self, key: FindKey) -> bool {
        if self.find_term().is_none() {
            return false;
        }
        let rows = self.panes[self.focused].grid.rows;
        let PaneContent::Terminal(t) = &mut self.panes[self.focused].content else {
            return false;
        };
        let Some(f) = t.find.as_mut() else {
            return false;
        };
        match key {
            FindKey::Close => t.find = None,
            FindKey::Other => {}
            FindKey::Char(c) => {
                f.edit(Some(c));
                f.refresh(&t.pty);
            }
            FindKey::Backspace => {
                f.edit(None);
                f.refresh(&t.pty);
            }
            FindKey::Older | FindKey::Newer => {
                f.refresh(&t.pty);
                f.step(key == FindKey::Older, t.pty.screen_line());
            }
        }
        if let Some(f) = &t.find {
            if let Some(to) = f.reveal(t.pty.display_offset(), rows, t.pty.history_lines()) {
                t.pty.scroll(to as i32 - t.pty.display_offset() as i32);
            }
        }
        self.redraw();
        true
    }

    /// Composed text (an IME commit) while the bar is open is the query's,
    /// not the program's. False when there is no bar.
    pub(crate) fn term_find_text(&mut self, text: &str) -> bool {
        if self.find_term().is_none() {
            return false;
        }
        text.chars().for_each(|c| {
            self.term_find_input(FindKey::Char(c));
        });
        true
    }

    /// Cmd+V: into the find query while the bar is open, else the pane.
    pub(crate) fn paste_or_find(&mut self) {
        if self.find_term().is_none() {
            return self.paste();
        }
        let text = crate::clipboard::system_text().unwrap_or_default();
        self.term_find_text(&text.replace(['\n', '\r'], ""));
    }
}

#[cfg(test)]
#[path = "termfindkeys_tests.rs"]
mod tests;
