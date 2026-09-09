//! The swarm status line's `3/7` counter, flashing when a task settles.
//!
//! The line's counter used to change in place — a `3` became a `4` between
//! two frames and, with the spinner turning and the elapsed clock ticking
//! beside it, nothing said that this change was the one that mattered. Now
//! the digit that changed brightens to the page ink on the change and eases
//! back to the muted words over [`FLASH_MS`] (Subtle: 60% of it; Off: no
//! flash — the counter simply reads its new value).
//!
//! State belongs to the surface that draws it (`readout`): the flash lives on
//! the run's [`crate::chatswarm::SwarmStatus`] and is observed at draw time
//! from the settled count, so it dies with the run and needs no hook in the
//! event fold. The first sight of a count is not a change: a plan's `0/7`
//! draws quiet.
use std::cell::Cell;

use crate::chat::ChatPane;
use crate::ease::Timeline;
use crate::shimmer::Color;

/// How long the changed digit stays lit, at full motion.
pub(crate) const FLASH_MS: u64 = 300;

/// One counter's flash: the count last drawn, the one before it, and the
/// window the change lit.
#[derive(Debug, Default)]
pub(crate) struct Flash {
    seen: Cell<Option<usize>>,
    prev: Cell<usize>,
    timeline: Cell<Timeline>,
}

impl Flash {
    /// The settled count is `done` at `now`: a change from the count seen
    /// last starts the flash; the first sight only records it.
    pub(crate) fn observe(&self, done: usize, now: u64) {
        match self.seen.get() {
            None => self.seen.set(Some(done)),
            Some(s) if s != done => {
                self.prev.set(s);
                self.seen.set(Some(done));
                self.timeline
                    .set(Timeline::start(now, FLASH_MS, crate::motion::level()));
            }
            Some(_) => {}
        }
    }

    /// How lit the changed digits are at `now`: 1 on the change, easing out
    /// to 0 by [`FLASH_MS`]; 0 once settled (and always, at Off).
    pub(crate) fn mix(&self, now: u64) -> f32 {
        let t = self.timeline.get();
        if !t.live(now) {
            return 0.0;
        }
        1.0 - t.eased(now, crate::ease::out_cubic)
    }

    /// Whether the flash still has frames to draw.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.timeline.get().live(now)
    }

    /// Which digits of `done` differ from the count before it, in order —
    /// every one when the number grew a digit (`9` → `10`).
    pub(crate) fn changed(&self, done: usize) -> Vec<bool> {
        let new = done.to_string();
        let old = self.prev.get().to_string();
        if new.len() != old.len() {
            return vec![true; new.len()];
        }
        new.chars().zip(old.chars()).map(|(a, b)| a != b).collect()
    }
}

/// The status line's words after the spinner — `rest`, muted — with the
/// settled count's changed digits lit by the run's flash at `now`. Every
/// colour is floored against the page. Without a live run the words are
/// plain muted.
pub(crate) fn words(pane: &ChatPane, rest: &str, now: u64) -> Vec<(char, Color)> {
    let th = crew_theme::theme();
    let muted = th.text_muted;
    let Some(s) = pane.swarm.as_ref() else {
        return rest.chars().map(|c| (c, muted)).collect();
    };
    let (done, total) = s.settled();
    s.flash.observe(done, now);
    let mix = s.flash.mix(now);
    // The count is the last `done/total` in the words: the title comes first
    // and nothing after the count carries a slash.
    let count = format!("{done}/{total}");
    let at = rest.rfind(&count).map(|b| rest[..b].chars().count());
    let changed = s.flash.changed(done);
    let lit = crew_theme::readable::against(
        crate::anim::lerp_rgb(muted, th.ink, mix),
        th.page_bg,
        crew_theme::contrast::text_floor(),
    );
    rest.chars()
        .enumerate()
        .map(|(i, c)| {
            let hot = mix > 0.0
                && at.is_some_and(|a| (a..a + changed.len()).contains(&i) && changed[i - a]);
            (c, if hot { lit } else { muted })
        })
        .collect()
}

#[cfg(test)]
#[path = "chatflash_tests.rs"]
mod tests;
