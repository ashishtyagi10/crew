//! Scrolling that travels instead of jumping.
//!
//! A wheel notch, Shift+PageUp/PageDown and Shift+Home/End used to move a
//! pane's scrollback in one frame: three rows, a page, or the whole buffer
//! gone between two frames, so the eye had nothing to follow and lost its
//! place. Now the offset rides a [`Spring`] — the grid glide's own primitive —
//! and the pane is handed whole rows as the spring passes them. A second
//! notch mid-flight adds to where it is going and keeps the speed it has,
//! which is what makes a run of notches one smooth motion, not a stutter.
//!
//! The offset itself stays in whole rows, so every reader of it (selection,
//! find, marks, pictures, hit-testing) keeps meaning what it meant. A
//! terminal is DRAWN at the fraction between them: the pane takes the row
//! ABOVE where the spring is (`ceil`), so the content only ever has to slide
//! UP by under a row, and the one line that uncovers is the line just under
//! the viewport — which the terminal hands over (`PtyTerm::row_below`) and
//! the renderer clips to the card (`crew_render::scrollclip`). Other panes
//! keep whole-row steps: their rows are built from their own offsets.
//!
//! Bounded: a critically damped spring converges and the snap ends it, so
//! [`crate::paneview::pane_animating`] goes quiet. At Motion off nothing is
//! queued; the scroll is the jump it always was. A trackpad scrolls a
//! terminal on a stiff spring ([`TRACK_OMEGA`]) — it brings its own momentum,
//! so the spring only spreads each row across a few frames for the drawing —
//! and every other pane directly, since those cannot draw the remainder.
use crate::pane::{Pane, PaneContent};
pub(crate) use crate::scrollstate::{ScrollGlide, OMEGA, TRACK_OMEGA};

/// Scroll `pane` by `lines`: queued on its glide while motion is on, at once
/// when it is off.
pub(crate) fn start(pane: &mut Pane, lines: i32) {
    start_at(pane, lines, OMEGA);
}

/// [`start`] at stiffness `omega`.
pub(crate) fn start_at(pane: &mut Pane, lines: i32, omega: f32) {
    if lines == 0 {
        return;
    }
    if crate::motion::level() == crate::motion::MotionLevel::Off {
        crate::scroll::scroll_pane(pane, lines);
        return;
    }
    if !pane.glide.scroll.moving() {
        pane.glide.scroll.mark = term_offset(pane);
        pane.glide.scroll.pixel = term_offset(pane).is_some();
    }
    pane.glide.scroll.push_at(lines, omega);
}

/// A trackpad's rows: a terminal glides on [`TRACK_OMEGA`] and is drawn
/// between them; any other pane takes them at once, as it always did.
pub(crate) fn start_track(pane: &mut Pane, lines: i32) {
    match term_offset(pane) {
        Some(_) => start_at(pane, lines, TRACK_OMEGA),
        None => crate::scroll::scroll_pane(pane, lines),
    }
}

/// Shift+Home/End. A terminal knows exactly how far its ends are, so the
/// trip is measured and glides; any other pane is asked to clamp a huge
/// delta, which only makes sense as the jump.
pub(crate) fn start_end(pane: &mut Pane, to_top: bool) {
    pane.glide.scroll.stop();
    let lines = match &pane.content {
        PaneContent::Terminal(t) => {
            let off = t.pty.display_offset() as i32;
            match to_top {
                true => t.pty.history_lines() as i32 - off,
                false => -off,
            }
        }
        _ => {
            let far = if to_top { i32::MAX / 2 } else { i32::MIN / 2 };
            return crate::scroll::scroll_pane(pane, far);
        }
    };
    start(pane, lines);
}

/// One frame of every pane's glide; whether any is still moving.
pub(crate) fn step_all(panes: &mut [Pane], dt_ms: u64) -> bool {
    for p in panes.iter_mut().filter(|p| p.glide.scroll.moving()) {
        let before = term_offset(p);
        // The terminal went back to its live bottom under us — the user
        // typed, or a program asked. Following through would scroll away
        // from what they are now looking at.
        if before == Some(0) && p.glide.scroll.mark.is_some_and(|m| m != 0) {
            p.glide.scroll.stop();
            continue;
        }
        let d = p.glide.scroll.step(dt_ms);
        if d != 0 {
            crate::scroll::scroll_pane(p, d);
        }
        let after = term_offset(p);
        // An end of the buffer: nothing left to travel through.
        if d != 0 && after == before && before.is_some() {
            p.glide.scroll.stop();
        }
        if p.glide.scroll.moving() {
            p.glide.scroll.mark = after;
        }
        // The line the slide uncovers, read now, for this frame's drawing.
        let between = p.glide.scroll.residual() != 0.0;
        p.glide.scroll.edge = match &mut p.content {
            PaneContent::Terminal(t) if between => t.pty.row_below(false),
            _ => None,
        };
    }
    panes.iter().any(|p| p.glide.scroll.moving())
}

fn term_offset(p: &Pane) -> Option<usize> {
    match &p.content {
        PaneContent::Terminal(t) => Some(t.pty.display_offset()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "scrollglide_tests.rs"]
mod tests;
