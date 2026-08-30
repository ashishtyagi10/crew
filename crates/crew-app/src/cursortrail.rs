//! The caret leaves a wake.
//!
//! A block cursor in a cell grid is a teleporting object: it is in one cell on
//! one frame and a different cell on the next, with nothing on the page saying
//! the two were the same thing. At a shell prompt that is invisible — you are
//! looking at the character you just typed — but the moment the cursor jumps
//! (a `Ctrl+A` to the line's head, a TUI moving a selection, a paste landing)
//! the eye has to *find* it again, because nothing on screen connects where it
//! was to where it is.
//!
//! So the caret drags a short streak behind it: a rectangle in the cursor's
//! own colour spanning the ground it just covered, fading out over about a
//! tenth of a second. Nothing is drawn *slower* — the cursor itself is exactly
//! where the program says it is on the very first frame, because a caret that
//! lags its own keystrokes is a worse terminal. The wake is a trace of the
//! move that already happened, not an animation of the move itself.
//!
//! It lives on the [`Paint`] layer — sub-cell rectangles under the text — so
//! the glyph the caret is sitting on stays readable through it, and it needs
//! no new plumbing through the scene: the terminal pane already carries paint.
//!
//! Bounded, like every animation here: one [`Timeline`] per pane, and only the
//! focused pane's is ever fed, so a background pane redrawing a spinner cannot
//! ask crew to repaint. At `Motion = off` the timeline is born settled and no
//! wake is ever drawn.
use crew_render::Paint;

use crate::ease::{out_cubic, Timeline};
use crate::motion;
use crate::pane::{Pane, PaneContent};

/// How long a wake takes to fade. Short on purpose: long enough to be *seen*
/// as motion, too short to still be there when the next keystroke lands at a
/// fast typing speed (~120ms between characters).
const WAKE_MS: u64 = 130;

/// Peak opacity of the wake, before its fade. The cursor block itself is
/// opaque; the streak has to read as its shadow, not as a second cursor.
const WAKE_ALPHA: f32 = 0.42;

/// The longest streak drawn, in cells. A cursor crossing 100 columns in one
/// frame (a full-width redraw) would otherwise flash a bar across the whole
/// pane — louder than the thing it is trying to point at. Longer jumps leave
/// a ghost on the cell the caret *left* instead, which says the same thing in
/// the space of one cell.
const MAX_STREAK: u16 = 24;

/// How many quads a streak is sliced into. Four is enough for the alpha ramp
/// to read as a taper and few enough that the whole wake costs less than a row
/// of text.
const SLICES: u8 = 4;

/// One pane's caret wake: the cell it came from, the cell it is at, and the
/// clock between them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Trail {
    from: Option<(u16, u16)>,
    at: Option<(u16, u16)>,
    since: Timeline,
}

impl Trail {
    /// Feed this frame's cursor cell. A move starts a fresh wake from where
    /// the caret was; standing still leaves the running one alone, so holding
    /// a key does not restart the fade on every repeat.
    pub(crate) fn observe(&mut self, cell: Option<(u16, u16)>, now: u64) {
        self.observe_at(cell, now, motion::level());
    }

    /// [`Trail::observe`] against an explicit motion level — the seam the
    /// reduce-motion test reads through, since the app-wide level is a global
    /// and the test suite runs in parallel.
    fn observe_at(&mut self, cell: Option<(u16, u16)>, now: u64, level: motion::MotionLevel) {
        if cell == self.at {
            return;
        }
        // A caret that just appeared (a program unhiding it, a scroll back to
        // the bottom) has come from nowhere — there is no ground to cover.
        self.from = match (self.at, cell) {
            (Some(prev), Some(_)) => Some(prev),
            _ => None,
        };
        self.at = cell;
        self.since = Timeline::start(now, WAKE_MS, level);
    }

    /// Whether this wake still has a frame to draw.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.from.is_some() && self.since.live(now)
    }

    /// The wake, in the pane's own cell coordinates: a short run of quads
    /// from the ground the caret covered, brightest against the caret and
    /// thinning away behind it. Empty once the fade is done.
    pub(crate) fn paint(&self, now: u64, color: (u8, u8, u8)) -> Vec<Paint> {
        let (Some(from), Some(at)) = (self.from, self.at) else {
            return Vec::new();
        };
        if !self.since.live(now) {
            return Vec::new();
        }
        // The tail catches up on an ease-out: most of the streak's length is
        // gone in its first third, which is what makes a wake read as *speed*
        // rather than as a rectangle politely dissolving.
        let t = self.since.eased(now, out_cubic);
        let alpha = WAKE_ALPHA * (1.0 - self.since.progress(now));
        streak(from, at, t, alpha, color)
    }
}

/// The streak between two cells at catch-up `t`, or a ghost on the departed
/// cell when the two are too far apart to join with a bar.
fn streak(from: (u16, u16), at: (u16, u16), t: f32, alpha: f32, color: (u8, u8, u8)) -> Vec<Paint> {
    let (dc, dr) = (span(from.0, at.0), span(from.1, at.1));
    if dc > MAX_STREAK || dr > 1 {
        // A ghost on the cell the caret left: the same statement in the space
        // of one cell, for a jump too long to draw a bar across.
        let (x, y) = (f32::from(from.0), f32::from(from.1));
        return vec![Paint::solid(x, y, 1.0, 1.0, color).at(alpha)];
    }
    // The tail slides toward the head; the head is always the cell the caret
    // is actually in, so the streak shrinks into it rather than sliding off.
    let tail = (lerp(from.0, at.0, t), lerp(from.1, at.1, t));
    let head = (f32::from(at.0), f32::from(at.1));
    // One quad would be a highlighted region — a block of colour with two hard
    // edges, which reads as *selection*, not as speed. Sliced along the travel
    // with the alpha ramping into the caret, the same rectangle reads as
    // something moving: dense where the caret is, almost gone where it was.
    (0..SLICES)
        .map(|k| {
            let (a, b) = (k as f32 / SLICES as f32, (k + 1) as f32 / SLICES as f32);
            let p0 = (mix(tail.0, head.0, a), mix(tail.1, head.1, a));
            let p1 = (mix(tail.0, head.0, b), mix(tail.1, head.1, b));
            let (x, w) = (p0.0.min(p1.0), (p1.0 - p0.0).abs() + 1.0);
            let (y, h) = (p0.1.min(p1.1), (p1.1 - p0.1).abs() + 1.0);
            Paint::solid(x, y, w, h, color).at(alpha * b)
        })
        .collect()
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn span(a: u16, b: u16) -> u16 {
    a.max(b) - a.min(b)
}

fn lerp(a: u16, b: u16, t: f32) -> f32 {
    let (a, b) = (f32::from(a), f32::from(b));
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Feed this frame's cursor position to the focused pane's wake, and settle
/// every other pane's.
///
/// Only one caret is ever followed: the unfocused panes draw a hollow cursor
/// (see `crew_term::cursor::shape_for`), and a background program stepping its
/// own cursor must never be a reason for crew to schedule a frame.
pub(crate) fn step(panes: &mut [Pane], focused: Option<usize>, now: u64) {
    for (i, p) in panes.iter_mut().enumerate() {
        if let PaneContent::Terminal(t) = &mut p.content {
            let cell = match focused == Some(i) {
                true => t.pty.cursor_cell(),
                false => None,
            };
            t.trail.observe(cell, now);
        }
    }
}

/// Whether any pane's wake still has a frame left — one term of
/// `wants_animation_frame`.
pub(crate) fn any_live(panes: &[Pane], now: u64) -> bool {
    panes.iter().any(|p| match &p.content {
        PaneContent::Terminal(t) => t.trail.live(now),
        _ => false,
    })
}

/// The wake to draw over this pane's cells — empty unless its caret just moved.
pub(crate) fn paint_for(p: &Pane, now: u64) -> Vec<Paint> {
    let PaneContent::Terminal(t) = &p.content else {
        return Vec::new();
    };
    let color = crew_theme::readable::cursor(crew_theme::theme(), true);
    t.trail.paint(now, color)
}

#[cfg(test)]
#[path = "cursortrail_tests.rs"]
mod cursortrail_tests;
