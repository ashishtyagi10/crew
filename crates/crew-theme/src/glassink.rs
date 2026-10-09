//! Every text role on see-through glass, floored over any desktop.
//!
//! A palette's roles were chosen on its PAGE. On see-through glass a pane
//! shows the desktop through its frost, and the desktop is anything: light
//! glass over a dark one turns mid-grey (≈134), where the greys and colours
//! picked to sit a quiet step off a near-white page all but vanished — dim
//! text 1.46:1, an error 1.86, the terminal's green 2.1 (2026-10-09). Only the ink,
//! near-black, held.
//!
//! So while the window is sheer, a glass palette is served with each role
//! walked toward its label pole (see [`crate::glasslegend::legible_on`]) until
//! it clears its floor over a black desktop AND a white one. The floors keep
//! the iPhone's ladder — primary, secondary, tertiary — rather than flatten
//! everything to black: body text AA, secondary text a step under it, hints
//! and quiet marks the UI floor. The tint (the accent family) keeps its own
//! colour, as the iPhone's blue does on every material.
use crate::glasslegend::{legible_on, pane_grounds};
use crate::readable::{MARK_FLOOR, TEXT_FLOOR};
use crate::Theme;

/// Secondary text — `text_muted` — sits between the ink and the hints: the
/// iPhone's secondary label measures about 3.6:1 on its own white.
pub const SECONDARY_FLOOR: f32 = 3.5;

/// `l`'s text roles floored over `t`'s glass on any desktop. `l` is the
/// palette being served (already lifted for the sheer window); nothing moves
/// off glass.
pub fn floor_roles(l: &mut Theme, t: &Theme) {
    let g = pane_grounds(t);
    if g.is_empty() {
        return;
    }
    let at = |want, floor| legible_on(t, want, floor, &g);
    l.ink = at(l.ink, TEXT_FLOOR);
    l.term_fg = at(l.term_fg, TEXT_FLOOR);
    l.text_muted = at(l.text_muted, SECONDARY_FLOOR);
    l.status_fg = at(l.status_fg, TEXT_FLOOR);
    // The alarm keeps its red: at the body's floor it went maroon on light
    // glass and pink on night. The iPhone's own red is about 3.6:1.
    l.bell = at(l.bell, SECONDARY_FLOOR);
    for role in [&mut l.dim, &mut l.placeholder, &mut l.hint_fg] {
        *role = at(*role, MARK_FLOOR);
    }
    // The accent family is left as the palette drew it: it is the glass's
    // tint — the cursor bar, the focus ring, the selection — far more than
    // it is text, and walked to the floor the iPhone's blue went navy.
    // The terminal's colours are text too — `ls`, `git status`, a
    // compiler's verdict: the body of what a shell pane says, so held to
    // the secondary floor rather than the hints' (a prompt read 3.16 on
    // light glass over a dark desktop). Slot 0 is the page's own colour
    // (black on night glass, as a fill) and is left to be one.
    for slot in l.ansi.iter_mut().skip(1) {
        *slot = at(*slot, SECONDARY_FLOOR);
    }
}

#[cfg(test)]
#[path = "glassink_tests.rs"]
mod tests;
