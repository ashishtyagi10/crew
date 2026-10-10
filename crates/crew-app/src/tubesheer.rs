//! The window's opacity: every theme is see-through glass (the user,
//! 2026-10-09: dark, light and CRT "with the same pattern as glass"), its
//! window capped at the theme's own `LiquidStyle::window` — the wallpaper a
//! tint over the desktop, each pane a smoked slab on top.
//!
//! A tube was a TINTED FACEPLATE at 84% until then: at a 12% veil (2026-10-06)
//! phosphor stood on a bright desktop with nothing behind it ("this crt theme
//! is ugly … please have proper color contrast"). It is glass now with the
//! dark mode's deeper smoke and the tube's text shadow, which is what holds
//! phosphor green over a white desktop (`liquid_tests` in crew-theme).
//!
//! Everything that reads the window's opacity reads
//! [`CrewApp::window_opacity`], so the page, the blur, the title bar, the
//! solid overlays and the lifted borders all agree on it. A lower Opacity %
//! still wins — that is a choice of transparency over contrast, made in
//! Settings.
use crate::app::CrewApp;

/// The window's opacity for a `setting` under theme `t`: capped at its
/// glass's own `window`.
pub(crate) fn sheer(setting: f32, t: &crew_theme::Theme) -> f32 {
    t.liquid.map_or(setting, |l| setting.min(l.window))
}

/// Who overrides an Opacity % of `setting` under `t`, and what the window
/// is instead: `("glass", 0.25)` on glass, `("tube", 0.35)` on a tube —
/// always, since every cap is under the setting's own floor. `None` when the
/// setting is what the window gets.
pub(crate) fn overridden(setting: f32, t: &crew_theme::Theme) -> Option<(&'static str, f32)> {
    let got = sheer(setting, t);
    if got >= setting - 1e-3 {
        return None;
    }
    Some((if t.is_tube() { "tube" } else { "glass" }, got))
}

impl CrewApp {
    /// The window's opacity this frame: the setting, capped under a tube or
    /// liquid glass ([`sheer`]).
    pub(crate) fn window_opacity(&self) -> f32 {
        sheer(self.config.window_opacity, crew_theme::theme())
    }

    /// Keep the window's sheer state in step with the theme. Run every frame
    /// (theme switches come from many paths), but it only reaches the
    /// renderer and AppKit when the opacity actually moves — a switch into or
    /// out of a tube.
    pub(crate) fn sync_window_opacity(&mut self) {
        let o = self.window_opacity();
        if self.applied_opacity == Some(o) {
            return;
        }
        self.applied_opacity = Some(o);
        self.apply_glass();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_caps_the_window_at_its_glass() {
        let low = crate::config::MIN_WINDOW_OPACITY;
        for id in crew_theme::ALL_THEMES {
            let t = id.theme();
            let cap = t.liquid.expect("every theme is glass").window;
            assert!(cap < 1.0, "{}: shows the desktop", id.as_str());
            assert_eq!(sheer(1.0, t), cap);
            // A lower Opacity % is transparency chosen over contrast: it wins.
            assert_eq!(sheer(low, t), low.min(cap));
        }
    }

    #[test]
    fn the_app_reads_the_cap_from_the_active_theme() {
        let _g = crate::app::theme_test_guard();
        let app = CrewApp::default();
        crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
        let tube = crew_theme::theme().liquid.unwrap().window;
        assert!(app.window_opacity() <= tube);
        assert_eq!(overridden(1.0, crew_theme::theme()), Some(("tube", tube)));
    }
}
