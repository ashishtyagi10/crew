//! The window's opacity: every theme is see-through glass (the user,
//! 2026-10-09: dark, light and CRT "with the same pattern as glass"), its
//! window set by the theme's own `LiquidStyle::window` — the page a
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
//! solid overlays and the lifted borders all agree on it. It is the theme's
//! alone: the Opacity % setting it once capped went when every theme's cap
//! fell under the setting's own floor (2026-10-09) and no value of it
//! changed the window any more.
use crate::app::CrewApp;

/// The window's opacity under theme `t`: its glass's own `window`, opaque
/// for a theme with none.
pub(crate) fn sheer(t: &crew_theme::Theme) -> f32 {
    t.liquid.map_or(1.0, |l| l.window)
}

impl CrewApp {
    /// The window's opacity this frame: the live theme's ([`sheer`]).
    pub(crate) fn window_opacity(&self) -> f32 {
        sheer(crew_theme::theme())
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
    fn every_theme_shows_the_desktop_through_its_glass() {
        for id in crew_theme::ALL_THEMES {
            let t = id.theme();
            let cap = t.liquid.expect("every theme is glass").window;
            assert!(cap < 1.0, "{}: shows the desktop", id.as_str());
            assert_eq!(sheer(t), cap);
        }
    }

    #[test]
    fn the_app_reads_its_opacity_from_the_active_theme() {
        let _g = crate::app::theme_test_guard();
        let app = CrewApp::default();
        for id in [
            crew_theme::ThemeId::CrtGreen,
            crew_theme::ThemeId::GlassClear,
        ] {
            crew_theme::set_theme(id);
            assert_eq!(app.window_opacity(), sheer(id.theme()));
        }
    }
}
