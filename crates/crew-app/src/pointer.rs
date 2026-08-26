//! The pointer's shape: what the thing under it can do.
//!
//! Crew never set a cursor icon, so the arrow looked identical over a shell's
//! output, over the `[x]` that kills it, and over a card that can be picked up
//! and carried. Every one of those is a different verb, and the pointer is
//! where an interface says so before anything is clicked.
//!
//! The mapping is a pure function of what is under the cursor, so it can be
//! read without a window; the app applies it once per pointer move, only when
//! the answer changes.
use winit::window::CursorIcon;

use crate::app::CrewApp;

/// What the pointer is over, most specific first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Over {
    /// Carrying a card (see [`crate::panedrag`]) — outranks everything, since
    /// the gesture continues wherever the pointer travels.
    Carrying,
    /// The sidebar's resize edge, or a pane's scroll gutter — both are
    /// one-dimensional handles, and both say so with a resize arrow.
    NavEdge,
    /// A pane's scroll gutter — a vertical handle, so a vertical arrow.
    Gutter,
    /// A control: a border button, a nav row, a strip thumbnail, the `+N`
    /// overflow tile.
    Control,
    /// A card's legend row — the handle a card is picked up by.
    Handle,
    /// Text: a pane's content, or the input bar.
    Text,
    /// The page itself.
    Page,
}

/// The icon for what is under the pointer.
pub(crate) fn icon(over: Over) -> CursorIcon {
    match over {
        // `Grabbing`, not `Grab`: this is a gesture already in progress.
        Over::Carrying => CursorIcon::Grabbing,
        Over::NavEdge => CursorIcon::ColResize,
        Over::Gutter => CursorIcon::RowResize,
        Over::Control => CursorIcon::Pointer,
        Over::Handle => CursorIcon::Grab,
        Over::Text => CursorIcon::Text,
        Over::Page => CursorIcon::Default,
    }
}

impl CrewApp {
    /// What the pointer is over right now.
    pub(crate) fn pointer_over(&self) -> Over {
        if self.card_drag.is_some() {
            return Over::Carrying;
        }
        if self.gutter_drag.is_some() {
            return Over::Gutter;
        }
        if self.cursor_on_nav_edge() {
            return Over::NavEdge;
        }
        if self.cursor_on_gutter() {
            return Over::Gutter;
        }
        // A toast is an overlay above everything, so it answers first — and
        // it is always clickable (open, or dismiss), so it is always a
        // control.
        if self.cursor_in && self.toasts.index_at(self.cursor.0, self.cursor.1).is_some() {
            return Over::Control;
        }
        if self.hover_btn().is_some()
            || self.pane_at_sidebar().is_some()
            || self.cursor_on_overflow()
        {
            return Over::Control;
        }
        // Inside a pane, content selects and the legend row carries: the same
        // split the mouse gestures already use, told the same way.
        match (self.pane_at_cursor(), self.cursor_any_cell()) {
            (Some(_), Some(_)) => Over::Text,
            (Some(_), None) => Over::Handle,
            (None, _) if self.cursor_in_input() => Over::Text,
            _ => Over::Page,
        }
    }

    /// Set the window's cursor to match, if it changed. Called on every
    /// pointer move, so the "if it changed" is what keeps it free.
    pub(crate) fn pointer_sync(&mut self) {
        let want = icon(self.pointer_over());
        if self.cursor_icon == want {
            return;
        }
        self.cursor_icon = want;
        if let Some(w) = &self.window {
            w.set_cursor(want);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{icon, Over};

    /// Every state must be told apart from the ones next to it — an icon table
    /// where two verbs share a shape says nothing.
    #[test]
    fn every_state_has_its_own_shape() {
        let all = [
            Over::Carrying,
            Over::NavEdge,
            Over::Gutter,
            Over::Control,
            Over::Handle,
            Over::Text,
            Over::Page,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(icon(*a), icon(*b), "{a:?} and {b:?} look the same");
            }
        }
    }

    /// A card already in hand reads as held, not as grabbable.
    #[test]
    fn carrying_and_grabbing_are_not_the_same_shape() {
        assert_eq!(icon(Over::Handle), winit::window::CursorIcon::Grab);
        assert_eq!(icon(Over::Carrying), winit::window::CursorIcon::Grabbing);
    }

    #[test]
    fn the_bare_page_leaves_the_pointer_alone() {
        assert_eq!(icon(Over::Page), winit::window::CursorIcon::Default);
    }
}
