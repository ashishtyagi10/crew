//! Which words of a long reply the box shows. By default the rows around the
//! caret (so typing is always seen); the wheel over the box moves the view
//! instead, to read a long reply back without moving the caret (the user,
//! 2026-11-25: "both panel should have scrollbar" - a thumb that cannot be
//! dragged by the wheel is half a scrollbar). The view is stamped with the
//! caret, the words and the wrap width and room it was set under: change any
//! (a key, a paste, a resize that re-wraps the words) and the stamp no longer
//! matches, so the box follows the caret again with no reset call at every
//! place that edits. A click in the words moves the caret on purpose and
//! restamps ([`keep`]), so the view stays where the reader put it.
use std::hash::{Hash, Hasher};

use super::compose::Compose;
use super::composedraw::{place, text_w};
use super::composegeo::geo;
use super::draw::wrap;
use super::InboxPane;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    top: usize,
    at: usize,
    words: u64,
    w: usize,
    room: usize,
}

fn stamp(c: &Compose) -> (usize, u64) {
    let mut h = std::hash::DefaultHasher::new();
    c.body.text.hash(&mut h);
    (c.body.at, h.finish())
}

/// The top wrapped row the box shows, of `total` rows with the caret on
/// `caret`, `room` rows showing. The ONE answer for the drawing and for a
/// click, so the line clicked is the line the caret lands on.
pub(crate) fn first(c: &Compose, total: usize, caret: usize, room: usize, w: usize) -> usize {
    let last = total.saturating_sub(room);
    let (at, words) = stamp(c);
    match c.view {
        Some(v) if (v.at, v.words, v.w, v.room) == (at, words, w, room) => v.top.min(last),
        _ => caret.saturating_sub(room.saturating_sub(1)),
    }
}

/// Hold the view at `top` under the caret as it now stands: a click moved the
/// caret, and the line clicked must stay where it was clicked.
pub(crate) fn keep(c: &mut Compose, top: usize, w: usize, room: usize) {
    let (at, words) = stamp(c);
    c.view = Some(View {
        top,
        at,
        words,
        w,
        room,
    });
}

impl InboxPane {
    /// The wheel over the reply box: its words move `lines` (positive =
    /// up). False when no reply is open.
    pub(crate) fn wheel_reply(&mut self, lines: i32, cols: u16, rows: u16) -> bool {
        let Some(b) = self.cards(cols, rows).reply else {
            return false;
        };
        let Some(c) = self.compose.as_mut() else {
            return false;
        };
        let w = text_w(b.w);
        let room = usize::from(geo(c, 0, b.h, w as u16).room);
        let total = wrap(&c.body.text, w).len();
        let (caret, _) = place(&c.body.text, c.body.at, w);
        let now = first(c, total, caret, room, w);
        let n = lines.unsigned_abs() as usize;
        let top = match lines > 0 {
            true => now.saturating_sub(n),
            false => (now + n).min(total.saturating_sub(room)),
        };
        keep(c, top, w, room);
        true
    }
}

#[cfg(test)]
#[path = "composeview_tests.rs"]
pub(super) mod tests;

#[cfg(test)]
#[path = "composeviewkey_tests.rs"]
mod key_tests;
