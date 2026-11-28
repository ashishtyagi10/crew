//! Which words of a long reply the box shows. By default the rows around the
//! caret (so typing is always seen); the wheel over the box moves the view
//! instead, to read a long reply back without moving the caret (the user,
//! 2026-11-25: "both panel should have scrollbar" - a thumb that cannot be
//! dragged by the wheel is half a scrollbar). The view is stamped with the
//! caret and the words it was set under: change either (a key, a click, a
//! paste) and the stamp no longer matches, so the box follows the caret again
//! with no reset call at every place that edits.
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
}

fn stamp(c: &Compose) -> (usize, u64) {
    let mut h = std::hash::DefaultHasher::new();
    c.body.text.hash(&mut h);
    (c.body.at, h.finish())
}

/// The top wrapped row the box shows, of `total` rows with the caret on
/// `caret`, `room` rows showing. The ONE answer for the drawing and for a
/// click, so the line clicked is the line the caret lands on.
pub(crate) fn first(c: &Compose, total: usize, caret: usize, room: usize) -> usize {
    let last = total.saturating_sub(room);
    let (at, words) = stamp(c);
    match c.view {
        Some(v) if v.at == at && v.words == words => v.top.min(last),
        _ => caret.saturating_sub(room.saturating_sub(1)),
    }
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
        let now = first(c, total, caret, room);
        let n = lines.unsigned_abs() as usize;
        let top = match lines > 0 {
            true => now.saturating_sub(n),
            false => (now + n).min(total.saturating_sub(room)),
        };
        let (at, words) = stamp(c);
        c.view = Some(View { top, at, words });
        true
    }
}

#[cfg(test)]
#[path = "composeview_tests.rs"]
mod tests;
