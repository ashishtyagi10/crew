//! The caret blinks when the program asks it to.
//!
//! vim's insert mode, zsh's vi-mode and Claude Code all send a blinking shape
//! (DECSCUSR 1/3/5, or DECSET 12), and crew drew every one of them steady.
//! Now the focused pane's cursor blinks the way Ghostty's, kitty's and
//! iTerm's do: solid for [`SOLID_MS`] after anything moves it or a key lands —
//! so typing never shows a missing caret — then dark and lit by turns every
//! [`PERIOD_MS`]. Only one caret ever blinks: an unfocused pane draws an
//! outline (`crew_term::cursor::shape_for`), and that stands still.
//!
//! It costs one frame per toggle and nothing else. The blink is not an
//! animation (`wants_animation_frame` would draw it at 15 fps); each frame
//! notes when the next toggle falls due and the poll asks for exactly one
//! repaint then ([`take_due`]). With motion off, or the window in the
//! background, the caret is steady and nothing is ever due.
use crate::pane::{Pane, PaneContent};

/// How long the caret stays lit after it moves or a key lands.
pub(crate) const SOLID_MS: u64 = 500;
/// Each half of the blink, dark or lit — the macOS insertion point's period.
pub(crate) const PERIOD_MS: u64 = 530;

/// Whether the caret is drawn at `now`, `last_activity` being the last time
/// it moved or a key landed. A steady shape, or blinking turned off by
/// motion, is always drawn.
pub(crate) fn visible(now: u64, last_activity: u64, blinking: bool, motion_on: bool) -> bool {
    if !blinking || !motion_on {
        return true;
    }
    match now.saturating_sub(last_activity).checked_sub(SOLID_MS) {
        None => true,
        // The solid stretch ends dark: that is the first toggle.
        Some(t) => (t / PERIOD_MS) % 2 == 1,
    }
}

/// When [`visible`] next changes its answer, or `None` when it never will.
pub(crate) fn next_toggle(
    now: u64,
    last_activity: u64,
    blinking: bool,
    motion_on: bool,
) -> Option<u64> {
    if !blinking || !motion_on {
        return None;
    }
    let solid_end = last_activity + SOLID_MS;
    Some(match now.checked_sub(solid_end) {
        None => solid_end,
        Some(t) => solid_end + (t / PERIOD_MS + 1) * PERIOD_MS,
    })
}

/// Whether carets may blink at all: motion on and the window in front.
pub(crate) fn on(win_focus: Option<bool>) -> bool {
    crate::motion::level() != crate::motion::MotionLevel::Off && win_focus.unwrap_or(true)
}

/// One pane's blink clock: what it last saw, when that last changed, and the
/// toggle the next frame is owed at.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Blink {
    /// The caret's cell and whether it blinks, as last seen; `None` until
    /// this pane is first watched (focusing a pane starts it solid).
    seen: Option<(Option<(u16, u16)>, bool)>,
    since: u64,
    due: Option<u64>,
}

impl Blink {
    /// Note this frame's caret and say whether to draw it. A move, a change
    /// of blink style, or a key (`input_ms`) restarts the solid stretch.
    pub(crate) fn observe(
        &mut self,
        now_seen: (Option<(u16, u16)>, bool),
        input_ms: u64,
        on: bool,
        now: u64,
    ) -> bool {
        if self.seen != Some(now_seen) {
            self.seen = Some(now_seen);
            self.since = now;
        }
        self.since = self.since.max(input_ms);
        // A hidden caret has nothing to blink and no frame to ask for.
        let blinking = now_seen.1 && now_seen.0.is_some();
        self.due = next_toggle(now, self.since, blinking, on);
        visible(now, self.since, blinking, on)
    }
}

/// Feed this frame's caret to the focused pane's blink clock and light or
/// darken it; every other pane is lit, its clock forgotten.
pub(crate) fn step(panes: &mut [Pane], focused: Option<usize>, input_ms: u64, on: bool, now: u64) {
    for (i, p) in panes.iter_mut().enumerate() {
        if let PaneContent::Terminal(t) = &mut p.content {
            let lit = match focused == Some(i) {
                true => {
                    let seen = (t.pty.cursor_cell(), t.pty.cursor_blinks());
                    t.trail.blink.observe(seen, input_ms, on, now)
                }
                false => {
                    t.trail.blink = Blink::default();
                    true
                }
            };
            t.pty.set_cursor_lit(lit);
        }
    }
}

/// Whether a caret's toggle has come due — the one repaint it is owed. The
/// due time is cleared here, so a frame that never lands (an occluded
/// window) cannot turn it into a stream; the next drawn frame sets it again.
pub(crate) fn take_due(panes: &mut [Pane], now: u64) -> bool {
    let mut due = false;
    for p in panes.iter_mut() {
        if let PaneContent::Terminal(t) = &mut p.content {
            if t.trail.blink.due.is_some_and(|at| now >= at) {
                t.trail.blink.due = None;
                due = true;
            }
        }
    }
    due
}

#[cfg(test)]
#[path = "cursorblink_tests.rs"]
mod tests;
