//! The list moving the way crew moves (goal 2026-10-20-inbox-list-look,
//! item 16): the selection's wash glides to the row it went to, a thread
//! archived or deleted folds away rather than vanishing, and new mail at the
//! top glows once in the accent. Each is a start time and a duration scaled
//! by crew's motion setting, so at Motion off every one is born settled and
//! draws its last state once: the list is exactly what it was before.
use super::wire::Thread;
use crate::motion::level;

/// How long each takes at full motion.
const GLIDE_MS: u64 = 140;
const FOLD_MS: u64 = 240;
const PULSE_MS: u64 = 1_200;

/// A thread on its way out of the list: drawn where it was, fading, a line
/// shorter as it goes, until the rows under it close up.
#[derive(Clone)]
pub(crate) struct Fold {
    /// Its place in the list, which the thread after it now holds.
    pub at: usize,
    pub thread: Thread,
    pub start: u64,
}

#[derive(Default)]
pub(crate) struct Motion {
    /// The selection left thread `.0` at `.1`.
    pub glide: Option<(usize, u64)>,
    pub fold: Option<Fold>,
    /// Threads new since the last list, and when they landed.
    pub fresh: Option<(Vec<String>, u64)>,
}

/// How far a motion `dur_ms` long that began at `start` has got, `0..1`, or
/// `None` once it is over (at once, at Motion off).
fn progress(start: u64, dur_ms: u64, now: u64) -> Option<f32> {
    let dur = level().scale_ms(dur_ms);
    let t = now.saturating_sub(start) as f32 / dur.max(1) as f32;
    (dur > 0 && t < 1.0).then_some(t)
}

impl Motion {
    /// Whether any of it still has a frame to draw.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.glide
            .is_some_and(|(_, s)| progress(s, GLIDE_MS, now).is_some())
            || self
                .fold
                .as_ref()
                .is_some_and(|f| progress(f.start, FOLD_MS, now).is_some())
            || self
                .fresh
                .as_ref()
                .is_some_and(|(_, s)| progress(*s, PULSE_MS, now).is_some())
    }

    /// The thread the selection is gliding from, and how far it has come.
    pub(crate) fn glide(&self, now: u64) -> Option<(usize, f32)> {
        let (from, start) = self.glide?;
        let t = progress(start, GLIDE_MS, now)?;
        Some((from, crate::ease::out_cubic(t)))
    }

    /// The thread folding away: where, what, the lines of a row of `lines`
    /// it still takes (a line goes at a time), and how faded it is, `0..1`.
    pub(crate) fn folding(&self, lines: u16, now: u64) -> Option<(usize, &Thread, u16, f32)> {
        let f = self.fold.as_ref()?;
        let t = progress(f.start, FOLD_MS, now)?;
        let left = (f32::from(lines) * (1.0 - t)).ceil() as u16;
        (left > 0).then_some((f.at, &f.thread, left, t))
    }

    /// How brightly thread `id` still glows, `0..1`, if it is new.
    pub(crate) fn glow(&self, id: &str, now: u64) -> Option<f32> {
        let (ids, start) = self.fresh.as_ref()?;
        let t = progress(*start, PULSE_MS, now)?;
        ids.iter()
            .any(|i| i == id)
            .then(|| 1.0 - crate::ease::out_cubic(t))
    }
}

/// The threads of a fresh first page `new` that are new since `old`: not
/// listed before, or with mail newer than when they were. None on the first
/// list, which is all new and none of it news.
pub(crate) fn fresh(old: &[Thread], new: &[Thread]) -> Vec<String> {
    if old.is_empty() {
        return Vec::new();
    }
    new.iter()
        .filter(|t| !old.iter().any(|o| o.id == t.id && o.at_ms >= t.at_ms))
        .map(|t| t.id.clone())
        .collect()
}

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;
