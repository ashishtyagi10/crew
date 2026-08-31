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
#[path = "panehover_tests.rs"]
mod tests;
