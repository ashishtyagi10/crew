//! Cinematic theme switches: instead of hard-cutting (or blanking behind a
//! solid wash, the old behavior), the renderer holds a snapshot of the last
//! old-theme frame and crossfades it over the fully-rendered new-theme frame
//! (~450ms) — `/theme`, the cycle hotkey, auto-rotation and an OS appearance
//! flip all read as one look melting into the next, never a blank screen.
//!
//! Detection is a per-frame diff of the active theme id (the same pattern as
//! focus bookkeeping): every one of the half-dozen switch paths is caught in
//! one place, none of them has to remember to stamp a timeline. The snapshot
//! and blend pass live renderer-side (`set_theme_fade`); at Motion off the
//! timeline is born settled and the switch is an instant cut.
use crate::app::CrewApp;
use crate::ease::Timeline;

/// How long the old frame takes to melt away.
const FADE_MS: u64 = 450;

impl CrewApp {
    /// Per-frame theme diff: stamp the fade timeline when the active theme
    /// changed since the last frame. The first frame ever only records the
    /// theme (`theme_seen` is `None`), so launch doesn't dip.
    pub(crate) fn theme_fade_tick(&mut self, now: u64) {
        let id = crew_theme::current_id();
        if self.theme_seen != Some(id) {
            if self.theme_seen.is_some() {
                self.theme_fade_anim = Timeline::start(now, FADE_MS, crate::motion::level());
            }
            self.theme_seen = Some(id);
        }
    }

    /// This frame's crossfade strength: how strongly the old frame still
    /// covers the new one, or `None` once settled (which is also the
    /// renderer's "stop drawing the snapshot" signal).
    pub(crate) fn theme_fade(&self, now: u64) -> Option<f32> {
        let t = self.theme_fade_anim.eased(now, crate::ease::out_cubic);
        (t < 1.0).then_some(1.0 - t)
    }
}

#[cfg(test)]
#[path = "themefade_tests.rs"]
mod tests;
