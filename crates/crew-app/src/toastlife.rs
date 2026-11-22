//! How long a toast lives: what removes one, and when one still wants a frame. Split from
//! `toast.rs` for the line cap; a child module, so it reads the stack's own fields.
use super::{Toasts, TTL_MS};

impl Toasts {
    /// Drop expired toasts. The drawn rects index the same list, so a prune
    /// that took any drops them too (the next frame records them afresh).
    pub(crate) fn prune(&mut self, now: u64) {
        let n = self.items.len();
        self.items.retain(|t| t.age(now) < TTL_MS);
        if self.items.len() != n {
            self.rects.clear();
        }
    }

    /// [`Self::prune`], saying whether a card went — the repaint it is owed. A
    /// stack held under the pointer has frozen ages only as of its last drawn
    /// frame, so it is left alone until the pointer moves.
    pub(crate) fn expire(&mut self, now: u64) -> bool {
        let n = self.items.len();
        if self.held_since.is_none() {
            self.prune(now);
        }
        self.items.len() != n
    }

    /// Whether any card has a frame left to draw *right now*
    /// ([`crate::toastexit::wants_frame`]). A resting toast asks for nothing —
    /// poll re-checks each tick, so frames resume when the exit window opens.
    pub(crate) fn any_live(&self, now: u64) -> bool {
        // A held stack asks for nothing: every age is frozen, so there is no
        // next frame to draw until the pointer moves — and a move repaints.
        if self.held_since.is_some() {
            return false;
        }
        let level = crate::motion::level();
        (self.items.iter())
            .any(|t| crate::toastexit::wants_frame(t.age(now), t.slide.live(now), level))
    }
}
