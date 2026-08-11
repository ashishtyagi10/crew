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
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn first_frame_records_without_dipping() {
        let mut app = CrewApp::default();
        app.theme_fade_tick(1_000);
        assert_eq!(app.theme_fade(1_001), None, "launch must not fade");
    }

    #[test]
    fn a_theme_change_starts_the_fade_then_it_decays_to_none() {
        let mut app = CrewApp::default();
        app.theme_fade_tick(1_000);
        // Simulate a switch by lying about what was seen — equivalent to the
        // active id changing between frames, without touching the global.
        let other = crew_theme::ALL_THEMES
            .into_iter()
            .find(|&t| Some(t) != app.theme_seen)
            .unwrap();
        app.theme_seen = Some(other);
        app.theme_fade_tick(2_000);
        let a0 = app.theme_fade(2_001).expect("fade up right after");
        assert!(a0 > 0.9, "starts near-full old frame, got {a0}");
        let a_mid = app.theme_fade(2_150).expect("still fading");
        assert!(a_mid < a0, "fade must decay: {a_mid} !< {a0}");
        assert_eq!(app.theme_fade(2_600), None, "settled past FADE_MS");
        // And the fade registers with the redraw scheduler while live.
        assert!(app.theme_fade_anim.live(2_100));
        assert!(!app.theme_fade_anim.live(2_600));
    }

    #[test]
    fn an_unchanged_theme_never_restamps() {
        let mut app = CrewApp::default();
        app.theme_fade_tick(1_000);
        app.theme_fade_tick(2_000);
        app.theme_fade_tick(3_000);
        assert_eq!(app.theme_fade(3_001), None);
    }
}
