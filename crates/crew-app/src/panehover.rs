//! What the pointer is over, published once per frame.
//!
//! A pane's `[-]` and `[x]` border buttons were click targets with no
//! affordance: three glyphs the same colour as the legend beside them, giving
//! no sign that the pointer had found them or which of the two it was on. This
//! is the state that lets [`crate::panecard`] light the one under the cursor.
//!
//! It rides an atomic, published by `build_frame` and read at the point of
//! use, for the same reason `panecardglow::IGNITE_T` does: the scene-building
//! call chain is already at clippy's argument limit, and hover is a property
//! of the *frame*, not of any one card.
use std::sync::atomic::{AtomicU32, Ordering};

use crate::app::CrewApp;

/// Which border button the pointer found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Btn {
    Min,
    Close,
}

impl Btn {
    fn code(self) -> u32 {
        match self {
            Btn::Min => 1,
            Btn::Close => 2,
        }
    }

    fn from_code(c: u32) -> Option<Btn> {
        match c {
            1 => Some(Btn::Min),
            2 => Some(Btn::Close),
            _ => None,
        }
    }
}

/// `slot << 2 | btn_code`, or 0 for "the pointer is on no button". `slot` is
/// the card's legend number ([`crate::panecard::Bar::index`]) — 0 when only
/// one card is on the canvas and it therefore carries no number.
static HOVER: AtomicU32 = AtomicU32::new(0);

/// The sidebar PANES row under the pointer, as `index + 1` (0 = none) — a
/// change detector only, so a pointer sweeping the nav repaints exactly once
/// per row it crosses.
static NAV: AtomicU32 = AtomicU32::new(0);

/// Publish the hovered sidebar row; `true` when it changed.
fn publish_nav(row: Option<usize>) -> bool {
    let v = row.map_or(0, |i| u32::try_from(i).unwrap_or(0).saturating_add(1));
    NAV.swap(v, Ordering::Relaxed) != v
}

fn encode(h: Option<(u16, Btn)>) -> u32 {
    match h {
        Some((slot, btn)) => (u32::from(slot) << 2) | btn.code(),
        None => 0,
    }
}

/// Publish this frame's hovered button. Returns `true` when it differs from
/// what was already published — the signal to schedule a redraw.
pub(crate) fn publish(h: Option<(u16, Btn)>) -> bool {
    HOVER.swap(encode(h), Ordering::Relaxed) != encode(h)
}

/// Read an encoded hover back for the card numbered `slot`. Cards other than
/// the hovered one get `None`, so exactly one button on the canvas ever lights.
fn decode(v: u32, slot: u16) -> Option<Btn> {
    ((v >> 2) as u16 == slot).then(|| Btn::from_code(v & 0b11))?
}

/// The button hovered on the card numbered `slot`, if any.
pub(crate) fn btn_for(slot: u16) -> Option<Btn> {
    decode(HOVER.load(Ordering::Relaxed), slot)
}

impl CrewApp {
    /// Which card's which button the cursor is over, in the `(slot, btn)`
    /// space [`btn_for`] reads. The slot mirrors how `paneview` numbers cards
    /// exactly — zoomed, or with a single pane, the one card has no number —
    /// so a lit button can never land on the wrong card.
    pub(crate) fn hover_btn(&self) -> Option<(u16, Btn)> {
        let (idx, btn) = match self.close_btn_at_cursor() {
            Some(i) => (i, Btn::Close),
            None => (self.min_btn_at_cursor()?, Btn::Min),
        };
        let slot = if self.zoomed || self.panes.len() < 2 {
            0
        } else {
            u16::try_from(idx + 1).unwrap_or(0)
        };
        Some((slot, btn))
    }

    /// Publish the frame's hover state (called from `build_frame`).
    pub(crate) fn publish_hover(&mut self) {
        publish(self.hover_btn());
    }

    /// Redraw when the pointer crossing the canvas changed what is lit —
    /// a border button, or the sidebar row under it. Called from
    /// `CursorMoved`, which otherwise only extends a drag: a frame per pixel
    /// of mouse travel would be a needless redraw storm.
    ///
    /// The sidebar row is only *tracked* here; `navcard::pane_rows` reads the
    /// live hit-test when it builds the rows, so there is one answer to
    /// "which row is under the pointer" and this is not it.
    pub(crate) fn hover_moved(&mut self) {
        let btn = publish(self.hover_btn());
        let nav = publish_nav(self.pane_at_sidebar());
        if btn || nav {
            self.redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{btn_for, decode, encode, publish, Btn};

    #[test]
    fn only_the_hovered_card_lights_a_button() {
        let v = encode(Some((3, Btn::Close)));
        assert_eq!(decode(v, 3), Some(Btn::Close));
        assert_eq!(decode(v, 2), None);
        assert_eq!(decode(v, 0), None);
    }

    #[test]
    fn the_unnumbered_card_is_slot_zero() {
        // Zoomed (or a lone pane): the one card carries no legend number, and
        // slot 0 must still be addressable rather than reading as "nothing".
        let v = encode(Some((0, Btn::Min)));
        assert_eq!(decode(v, 0), Some(Btn::Min));
        assert_eq!(decode(v, 1), None);
    }

    #[test]
    fn nothing_hovered_lights_nothing_anywhere() {
        let v = encode(None);
        for slot in 0..4 {
            assert_eq!(decode(v, slot), None);
        }
    }

    #[test]
    fn the_two_buttons_never_encode_alike() {
        assert_ne!(encode(Some((1, Btn::Min))), encode(Some((1, Btn::Close))));
        assert_ne!(encode(Some((1, Btn::Min))), encode(Some((2, Btn::Min))));
        assert_ne!(encode(Some((0, Btn::Min))), encode(None));
    }

    /// Sidebar rows get their own change detector; the button one must not
    /// answer for them (a pointer sweeping the nav crosses no buttons at all,
    /// and would otherwise never repaint).
    #[test]
    fn the_nav_row_is_tracked_separately_from_the_buttons() {
        super::publish_nav(None);
        assert!(super::publish_nav(Some(0)), "onto the first row");
        assert!(!super::publish_nav(Some(0)), "still on it");
        assert!(super::publish_nav(Some(1)), "onto the next row");
        assert!(super::publish_nav(None), "off the list");
        assert!(!super::publish_nav(None), "and stays off");
    }

    /// The only test that touches the shared button atomic, so it can never race
    /// another one in this binary.
    #[test]
    fn publish_reports_only_real_changes_and_round_trips() {
        publish(None);
        assert!(publish(Some((1, Btn::Min))), "none -> min is a change");
        assert_eq!(btn_for(1), Some(Btn::Min), "and it is readable");
        assert!(!publish(Some((1, Btn::Min))), "same target is not");
        assert!(publish(Some((1, Btn::Close))), "other button on same card");
        assert!(publish(Some((2, Btn::Close))), "same button on other card");
        assert_eq!(btn_for(1), None, "the card it left goes dark");
        assert!(publish(None), "leaving the button is a change");
        assert_eq!(btn_for(2), None);
    }
}
