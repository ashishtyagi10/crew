//! The `↓ N new` pill's pop: a beat of emphasis each time N grows.
//!
//! The pill sits at the bottom right of a scrolled-up transcript, and a
//! message landing out of view only ever changed its number. Now each
//! increase inverts the pill — the page ink on an accent block — for
//! [`POP_MS`], then it settles back to its resting style. Off never pops.
//!
//! Observed at draw time from the pane's `unread`, like the readouts: the
//! state belongs to the surface that draws it. The pill is only drawn while
//! scrolled up, which is also the only time `unread` grows, so the draw sees
//! every increase; the scroll back to the bottom resets it ([`Pop::reset`]).
use std::cell::Cell;

use crate::ease::Timeline;

/// How long the pill stays inverted after N grows, at full motion.
pub(crate) const POP_MS: u64 = 200;

/// The pill's pop: the count last drawn and the window the increase lit.
#[derive(Debug, Default)]
pub(crate) struct Pop {
    seen: Cell<usize>,
    timeline: Cell<Timeline>,
}

impl Pop {
    /// The pill shows `unread` at `now`; whether it is popped right now. An
    /// increase over the count seen last starts the pop; a decrease is just
    /// recorded.
    pub(crate) fn observe(&self, unread: usize, now: u64) -> bool {
        if unread > self.seen.get() {
            self.timeline
                .set(Timeline::start(now, POP_MS, crate::motion::level()));
        }
        self.seen.set(unread);
        self.live(now)
    }

    /// Whether the pop still has frames to draw.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.timeline.get().live(now)
    }

    /// Nothing is unread any more (the view is back at the live bottom): the
    /// next message out of view is an increase again.
    pub(crate) fn reset(&self) {
        self.seen.set(0);
    }
}

#[cfg(test)]
#[path = "chatpop_tests.rs"]
mod tests;
