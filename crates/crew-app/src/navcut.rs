//! Which texts the nav's last panel draw had to cut, and on which row — what the hover card
//! (`navtip`) asks before it offers a row's whole text.
//!
//! A card that repeated a row the nav already showed whole would be noise, so the card needs
//! to know whether the row was cut. Re-deriving that from widths beside each section's draw
//! would be a second copy of every row's layout, free to drift from the one on screen. So the
//! one clipping call every nav row goes through ([`crate::navtext::put_at`], and the PANES
//! title's [`clip`]) notes what it shortened, while [`record`] is listening: the frame's own
//! draw is the answer. The row is part of it: two Far folders under one parent draw the same
//! `~/code`, and the long-named one's cut parent says nothing about the short one's row.
use std::cell::RefCell;

/// A text the draw cut, on the panel's content row it was drawn on.
pub(crate) type Cut = (u16, String);

thread_local! {
    /// The texts cut since [`record`] began, or `None` when nothing is listening.
    static CUTS: RefCell<Option<Vec<Cut>>> = const { RefCell::new(None) };
}

/// Run `draw`, and return what it drew with every text it had to cut on the way.
pub(crate) fn record<T>(draw: impl FnOnce() -> T) -> (T, Vec<Cut>) {
    let outer = CUTS.with(|c| c.replace(Some(Vec::new())));
    let out = draw();
    let cut = CUTS.with(|c| c.replace(outer)).unwrap_or_default();
    (out, cut)
}

/// Run `draw`, whose rows the caller moves to `to(row)` once drawn (the PANES list under its
/// rule, a tab card's pane row), and note its cuts on the rows they end up on.
pub(crate) fn moved<T>(to: impl Fn(u16) -> u16, draw: impl FnOnce() -> T) -> T {
    let (out, cut) = record(draw);
    for (row, s) in cut {
        note(to(row), &s);
    }
    out
}

/// Note that `s`, on `row`, was not drawn whole — cut, or left out for want of room.
pub(crate) fn note(row: u16, s: &str) {
    CUTS.with(|c| {
        if let Some(v) = c.borrow_mut().as_mut() {
            v.push((row, s.to_string()));
        }
    });
}

/// [`crate::chatwidth::clip_words`], noting `s` on `row` when it comes back shorter.
pub(crate) fn clip(row: u16, s: &str, max: usize) -> String {
    let fit = crate::chatwidth::clip_words(s, max);
    if fit != s {
        note(row, s);
    }
    fit
}

/// `s` in `room` columns: whole, or its END after `…` — two folders' paths differ at the end,
/// so a cut from the end left both twins reading `/var/folders…`.
pub(crate) fn tail(s: &str, room: usize) -> String {
    if crate::chatwidth::str_w(s) <= room {
        return s.to_string();
    }
    let mut kept: Vec<char> = Vec::new();
    let mut w = 1; // the ellipsis
    for c in s.chars().rev() {
        w += crate::chatwidth::char_w(c);
        if w > room {
            break;
        }
        kept.push(c);
    }
    std::iter::once('\u{2026}')
        .chain(kept.into_iter().rev())
        .collect()
}
