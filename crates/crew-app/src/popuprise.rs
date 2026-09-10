//! A composer pop-up rises out of the composer. It used to appear whole on
//! the frame its state opened — a card the size of a hand, there between
//! one keystroke and the next, with nothing to say where it came from. Now
//! its first frames draw it half a row low, inside the composer's top edge,
//! easing up to where it stands: it grows out of the field it completes,
//! which is what a dropdown is. Off with the motion gate, like every other
//! motion; short enough (140 ms) that it never delays a pick.
use crate::ease::Timeline;

/// Whether a pop-up was drawn last frame, and the rise that began when one
/// opened. `Default` is settled: nothing open, no rise.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rise {
    open: bool,
    tl: Timeline,
}

/// How long the rise takes at full motion.
pub(crate) const RISE_MS: u64 = 140;
/// How far below its place the card starts, in rows: half a row, so its
/// bottom edge begins on the composer's border rather than under it.
const DROP_ROWS: f32 = 0.5;

impl Rise {
    /// The frame's tick: `open` is whether a pop-up is drawn this frame. The
    /// frame one opens on starts the rise; staying open keeps it; closing
    /// and reopening replays it.
    pub(crate) fn tick(&mut self, open: bool, now: u64) {
        self.tick_at(open, now, crate::motion::level());
    }

    /// [`Self::tick`] at an explicit motion level — the seam the tests use,
    /// so none of them has to touch the process-wide gate.
    pub(crate) fn tick_at(&mut self, open: bool, now: u64, level: crate::motion::MotionLevel) {
        if open && !self.open {
            self.tl = Timeline::start(now, RISE_MS, level);
        }
        self.open = open;
    }

    /// Still rising — frames are owed.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.open && self.tl.live(now)
    }

    /// Rows the card is still short of its place: [`DROP_ROWS`] on the
    /// frame it opens, easing to zero. Zero when nothing is open.
    pub(crate) fn drop_rows(&self, now: u64) -> f32 {
        if !self.open {
            return 0.0;
        }
        (1.0 - self.tl.eased(now, crate::ease::out_cubic)) * DROP_ROWS
    }
}

#[cfg(test)]
#[path = "popuprise_tests.rs"]
mod tests;
