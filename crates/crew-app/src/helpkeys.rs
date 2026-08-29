//! Keys while the `/keys` overlay is open: moving through the list, and
//! filtering it by typing. Split from [`crate::help`] (the data model is in
//! [`crate::helplayout`], the drawing in `help`) for the 200-line cap.
use crate::help::{max_scroll, size};

/// How many rows a page key moves, so a long list is a few presses rather
/// than thirty.
const PAGE: i32 = 8;

impl crate::app::CrewApp {
    /// Show the keys overlay, from the top of an unfiltered list.
    pub(crate) fn open_help(&mut self) {
        self.help_open = true;
        self.help_scroll = 0;
        self.help_filter.clear();
    }

    /// Put it away, forgetting where it was read to and what was typed into
    /// it — the next `Cmd+/` is a fresh question, not the last one resumed.
    pub(crate) fn close_help(&mut self) {
        self.help_open = false;
        self.help_scroll = 0;
        self.help_filter.clear();
    }

    /// The rows a key moves the open help by, or `None` when the key means
    /// "close" — which is every key that is not a way of moving through a
    /// list, so the overlay keeps its press-anything-to-dismiss habit.
    pub(crate) fn help_scroll_step(&self, key: &winit::keyboard::Key) -> Option<i32> {
        use winit::keyboard::{Key, NamedKey};
        match key {
            Key::Named(NamedKey::ArrowDown) => Some(1),
            Key::Named(NamedKey::ArrowUp) => Some(-1),
            Key::Named(NamedKey::PageDown) => Some(PAGE),
            Key::Named(NamedKey::PageUp) => Some(-PAGE),
            Key::Named(NamedKey::End) => Some(i32::MAX),
            Key::Named(NamedKey::Home) => Some(i32::MIN),
            _ => None,
        }
    }

    /// One key press while the overlay is open.
    ///
    /// Typing filters rather than dismissing: with forty-odd bindings the list
    /// is a document, and the fastest way through a document is to say what
    /// you are looking for. That leaves Esc (and Enter) to close it, which is
    /// what a person who typed something expects anyway; every other key still
    /// closes, so nothing traps you in here.
    pub(crate) fn help_key(&mut self, key: &winit::keyboard::Key) {
        use winit::keyboard::{Key, NamedKey};
        if let Some(step) = self.help_scroll_step(key) {
            self.scroll_help(step);
            return;
        }
        match key {
            Key::Character(c) if !c.is_empty() && !c.chars().any(char::is_control) => {
                self.help_filter.push_str(c);
                // A narrower list read from wherever the old one was scrolled
                // to would open on nothing.
                self.help_scroll = 0;
                return;
            }
            Key::Named(NamedKey::Space) => {
                self.help_filter.push(' ');
                self.help_scroll = 0;
                return;
            }
            Key::Named(NamedKey::Backspace) => {
                self.help_filter.pop();
                self.help_scroll = 0;
                return;
            }
            _ => {}
        }
        self.help_open = false;
        self.help_scroll = 0;
        self.help_filter.clear();
    }

    /// Move the open help by `step` rows, clamped to its list.
    pub(crate) fn scroll_help(&mut self, step: i32) {
        let rows = self
            .frame_geometry()
            .map_or(size().1, |(_, ch, _, sh, _)| (sh / ch) as u16)
            .min(size().1);
        let cols = self
            .frame_geometry()
            .map_or(size().0, |(cw, _, sw, _, _)| (sw / cw) as u16)
            .min(size().0);
        let max = max_scroll(rows, cols, &self.help_filter) as i64;
        let want = self.help_scroll as i64 + i64::from(step);
        self.help_scroll = want.clamp(0, max) as usize;
    }
}

#[cfg(test)]
#[path = "helpkeys_tests.rs"]
mod tests;
