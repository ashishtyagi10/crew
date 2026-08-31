//! Pane attention markers: a "needs you" flag raised on a pane by notification
//! events (bell, watched pattern, command finished) while you're not looking at
//! it — so a minimized or unfocused pane can flag for input in the nav. Pure
//! timing/state helpers; raising lives in `poll`, clearing in `render`, and
//! drawing in `panelist`/`minstrip`.
use crate::notify::NotifyKind;
use crate::pane::Pane;

/// Blink half-period while pulsing: the marker toggles every `BLINK_MS`.
pub const BLINK_MS: u64 = 400;
/// How long a fresh marker blinks before settling into a steady glyph. Redraws
/// are only driven inside this window, so an ignored marker costs nothing.
pub const PULSE_MS: u64 = 4000;

/// A raised marker: what happened and when (on the shared `anim` clock).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attention {
    pub kind: NotifyKind,
    pub at_ms: u64,
}

impl Attention {
    /// The one-cell marker drawn on the pane's nav row / thumbnail.
    pub fn glyph(&self) -> char {
        match self.kind {
            NotifyKind::Bell => '!',
            NotifyKind::Pattern => '⚑',
            NotifyKind::AgentDone => '✓',
            NotifyKind::Failed => '✗',
            NotifyKind::Exited => '⊗',
            NotifyKind::Waiting => '?',
            NotifyKind::Requested => '\u{25b8}',
        }
    }

    /// Still inside the blink window (drives redraws)?
    ///
    /// Never, with motion off: a marker that does not blink does not need the
    /// frames, and the poll loop keeps redrawing only while something is
    /// pulsing.
    pub fn pulsing(&self, now: u64) -> bool {
        !steady() && now.saturating_sub(self.at_ms) < PULSE_MS
    }

    /// Is the marker drawn at `now`? Blinks during the pulse, steady after.
    ///
    /// With motion off it is simply *there*, from the first frame. Reduce
    /// motion is an accessibility request and blinking is the one kind of
    /// motion it is most specifically about — and the marker loses nothing by
    /// standing still, because what it has to say is that it exists.
    pub fn visible(&self, now: u64) -> bool {
        if steady() {
            return true;
        }
        let dt = now.saturating_sub(self.at_ms);
        dt >= PULSE_MS || (dt / BLINK_MS).is_multiple_of(2)
    }
}

/// Whether markers hold still instead of blinking — motion `Off`, which is
/// also what `auto` resolves to when the OS is asking for reduced motion.
fn steady() -> bool {
    crate::motion::level() == crate::motion::MotionLevel::Off
}

/// Raise a marker on `p` at `now`. The newest event wins (restarts the pulse).
pub fn raise(p: &mut Pane, kind: NotifyKind, now: u64) {
    p.attention = Some(Attention { kind, at_ms: now });
}

/// True while any pane's marker is still blinking — the poll loop keeps
/// redrawing only then, so a settled marker never costs a frame.
pub fn any_pulsing(panes: &[Pane], now: u64) -> bool {
    panes
        .iter()
        .any(|p| p.attention.is_some_and(|a| a.pulsing(now)))
}

#[cfg(test)]
#[path = "attention_tests.rs"]
mod tests;
