//! Picking a card up by its top border and dropping it on another one.
//!
//! Panes have always been reorderable — `Cmd+{` / `Cmd+}` by index,
//! `Cmd+Shift+Arrow` by direction — but only from the keyboard. The canvas is
//! a set of cards on a surface and behaves like one everywhere else, so the
//! obvious thing to try was to drag one, and the obvious thing did nothing.
//!
//! The grab region is the top border: the row carrying the legend, the same
//! row a window's title bar occupies and the same row whose double-click
//! zooms (see [`crate::selrun`]). It is also, conveniently, the one row of a
//! card that holds no text to select — so a card drag and a text drag can
//! never both be armed.
use std::sync::atomic::{AtomicU32, Ordering};

use crate::app::CrewApp;

/// How far the pointer must travel before a press on a legend becomes a drag
/// rather than a click. Below this a hand that shakes on a double-click would
/// start swapping panes.
const THRESHOLD: f32 = 6.0;

/// A card picked up but not yet dropped.
#[derive(Clone, Copy)]
pub(crate) struct CardDrag {
    pane: usize,
    from: (f32, f32),
    /// Whether the pointer has travelled past [`THRESHOLD`] — until it has,
    /// this is still a click.
    moved: bool,
}

/// Whether the pointer has travelled far enough from where it was pressed for
/// the gesture to be a drag.
fn past_threshold(from: (f32, f32), now: (f32, f32)) -> bool {
    (now.0 - from.0).abs().max((now.1 - from.1).abs()) >= THRESHOLD
}

/// The two pane indices a release should swap, or `None` when the gesture was
/// not a drag, landed nowhere, landed back on the card it started from, or
/// names a pane that has since closed.
fn swap_for(d: CardDrag, over: Option<usize>, n: usize) -> Option<(usize, usize)> {
    if !d.moved {
        return None;
    }
    let target = over.filter(|&t| t != d.pane)?;
    (target < n && d.pane < n).then_some((d.pane, target))
}

/// The card the pointer is over mid-drag, as legend number (0 = none). Read by
/// [`crate::panecard`] at draw time, like the hover state next to it.
static DROP: AtomicU32 = AtomicU32::new(0);

fn publish_drop(slot: u32) -> bool {
    DROP.swap(slot, Ordering::Relaxed) != slot
}

/// Whether the card numbered `slot` is the one a drag would land on. Slot 0 —
/// the unnumbered lone card — never is: a swap needs two cards.
pub(crate) fn is_drop_target(slot: u16) -> bool {
    slot != 0 && DROP.load(Ordering::Relaxed) == u32::from(slot)
}

impl CrewApp {
    /// A left press landed on pane `i` and armed no text selection, so it
    /// found the card's legend row: pick the card up.
    pub(crate) fn card_press(&mut self, i: usize) {
        if self.drag.is_some() || self.panes.len() < 2 || self.zoomed {
            return;
        }
        self.card_drag = Some(CardDrag {
            pane: i,
            from: self.cursor,
            moved: false,
        });
    }

    /// Cursor moved with a card in hand: light the card underneath it.
    /// Returns `true` when the frame should be redrawn.
    pub(crate) fn card_drag_move(&mut self) -> bool {
        let Some(d) = self.card_drag else {
            return false;
        };
        if !d.moved && !past_threshold(d.from, self.cursor) {
            return false;
        }
        if let Some(c) = self.card_drag.as_mut() {
            c.moved = true;
        }
        let over = self
            .pane_at_cursor()
            .filter(|&t| t != d.pane)
            .map_or(0, |t| t as u32 + 1);
        publish_drop(over)
    }

    /// Release: swap the carried card with whatever it was dropped on.
    /// Returns `true` when a swap happened, so the caller knows the gesture
    /// was a drag and not a click.
    pub(crate) fn card_drop(&mut self) -> bool {
        let Some(d) = self.card_drag.take() else {
            return false;
        };
        publish_drop(0);
        let Some((a, b)) = swap_for(d, self.pane_at_cursor(), self.panes.len()) else {
            return false;
        };
        self.panes.swap(a, b);
        self.focused = b;
        self.input.focused = false;
        true
    }
}

#[cfg(test)]
#[path = "panedrag_tests.rs"]
mod tests;
