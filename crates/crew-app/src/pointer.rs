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
    /// A URL or file reference drawn as a link ([`crate::linkhover`]) — one
    /// modifier away from opening a browser or a viewer, and until now
    /// wearing the same I-beam as the prose beside it.
    Link,
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
        // The same hand a control gets: both are "this does something when
        // you press it", and inventing a third shape for the distinction
        // would be a shape nobody has learned.
        Over::Link => CursorIcon::Pointer,
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
            (Some(_), Some(_)) if crate::linkhover::any() => Over::Link,
            (Some(_), Some(_)) => Over::Text,
            (Some(_), None) => Over::Handle,
            (None, _) if self.cursor_in_input() => Over::Text,
            _ => Over::Page,
        }
    }

    /// Set the window's cursor to match, if it changed. Called on every
    /// pointer move, so the "if it changed" is what keeps it free.
    pub(crate) fn pointer_sync(&mut self) {
        // The link under the pointer decides the shape, so it is answered
        // BEFORE the shape is asked for — and a run that moved repaints, since
        // the hovered run's weight is part of the frame.
        if self.link_hover_sync() {
            self.redraw();
        }
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
    /// `Link` and `Control` deliberately SHARE the hand: both are "this does
    /// something when you press it", and a third shape for the distinction
    /// would be one nobody has learned. Everything else must be distinct.
    #[test]
    fn a_link_wears_the_same_hand_a_control_does() {
        assert_eq!(icon(Over::Link), icon(Over::Control));
        assert_ne!(icon(Over::Link), icon(Over::Text));
    }

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
