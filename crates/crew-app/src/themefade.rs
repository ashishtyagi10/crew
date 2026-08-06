//! Cinematic theme switches: instead of hard-cutting, the new theme's page
//! washes the whole window and fades away (~450ms), so `/theme`, the cycle
//! hotkey, auto-rotation and an OS appearance flip all read as the page
//! *developing* its new look rather than the screen being swapped.
//!
//! Detection is a per-frame diff of the active theme id (the same pattern as
//! focus bookkeeping): every one of the half-dozen switch paths is caught in
//! one place, none of them has to remember to stamp a timeline. The veil
//! itself is one full-window quad the renderer draws last (`set_veil`); at
//! Motion off the timeline is born settled and no veil ever shows.
use crate::app::CrewApp;
use crate::ease::Timeline;

/// How long the new page takes to develop.
const VEIL_MS: u64 = 450;

impl CrewApp {
    /// Per-frame theme diff: stamp the veil timeline when the active theme
    /// changed since the last frame. The first frame ever only records the
    /// theme (`theme_seen` is `None`), so launch doesn't dip.
    pub(crate) fn theme_fade_tick(&mut self, now: u64) {
        let id = crew_theme::current_id();
        if self.theme_seen != Some(id) {
            if self.theme_seen.is_some() {
                self.theme_veil = Timeline::start(now, VEIL_MS, crate::motion::level());
            }
            self.theme_seen = Some(id);
        }
    }

    /// The veil to draw this frame: the active page color at the fade's
    /// remaining strength, or `None` once settled (which is also the renderer's
    /// "clear the quad" signal).
    pub(crate) fn theme_veil(&self, now: u64) -> Option<((u8, u8, u8), f32)> {
        let t = self.theme_veil.eased(now, crate::ease::out_cubic);
        (t < 1.0).then(|| (crew_theme::theme().page_bg, 1.0 - t))
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn first_frame_records_without_dipping() {
        let mut app = CrewApp::default();
        app.theme_fade_tick(1_000);
        assert_eq!(app.theme_veil(1_001), None, "launch must not veil");
    }

    #[test]
    fn a_theme_change_raises_the_veil_then_it_decays_to_none() {
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
        let (color, a0) = app.theme_veil(2_001).expect("veil up right after");
        assert_eq!(color, crew_theme::theme().page_bg);
        assert!(a0 > 0.9, "starts near-opaque, got {a0}");
        let (_, a_mid) = app.theme_veil(2_150).expect("still fading");
        assert!(a_mid < a0, "fade must decay: {a_mid} !< {a0}");
        assert_eq!(app.theme_veil(2_600), None, "settled past VEIL_MS");
        // And the veil registers with the redraw scheduler while live.
        assert!(app.theme_veil.live(2_100));
        assert!(!app.theme_veil.live(2_600));
    }

    #[test]
    fn an_unchanged_theme_never_restamps() {
        let mut app = CrewApp::default();
        app.theme_fade_tick(1_000);
        app.theme_fade_tick(2_000);
        app.theme_fade_tick(3_000);
        assert_eq!(app.theme_veil(3_001), None);
    }
}
