//! A CRT tube is a terminal running in glass, so its window is never a black
//! slab (2026-10-06, the user: "we don't need background swirl in the crt
//! mode, just the glass and borders, it should be almost frosted glass, no
//! black, like transparent").
//!
//! Under a tube theme the window is always SHEER — the desktop frosted behind
//! it by the window server's blur (`titlebar::apply_window`) — whatever the
//! Opacity % setting says (its floor sits far above a tube's veil, and the
//! title bar frosts with it: `titlebar::wanted`). Everything that
//! reads the window's opacity reads [`CrewApp::window_opacity`], so the page,
//! the blur, the title bar, the solid overlays and the lifted borders all
//! agree on it.
use crate::app::CrewApp;

/// How opaque a tube's page is at most: a faint veil, so the desktop shows
/// almost untouched (only frosted) between the panes, and the panes' own
/// milky frost (crew-theme's `tube_glass`) is barely more. 0.45 with dark
/// smoke on the panes still read as a dark window (2026-10-06, the user:
/// "still not transparent enough, I said frosty glass, they are still dark").
pub(crate) const TUBE_OPACITY: f32 = 0.12;

/// The window's opacity for a `setting` under a theme that is (`tube`) or is
/// not a tube.
pub(crate) fn sheer(setting: f32, tube: bool) -> f32 {
    if tube {
        setting.min(TUBE_OPACITY)
    } else {
        setting
    }
}

impl CrewApp {
    /// The window's opacity this frame: the setting, capped at
    /// [`TUBE_OPACITY`] under a tube.
    pub(crate) fn window_opacity(&self) -> f32 {
        sheer(self.config.window_opacity, crew_theme::theme().is_tube())
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
    fn a_tube_is_always_sheer_and_other_themes_keep_the_setting() {
        assert_eq!(sheer(1.0, true), TUBE_OPACITY);
        assert_eq!(sheer(crate::config::MIN_WINDOW_OPACITY, true), TUBE_OPACITY);
        assert_eq!(sheer(1.0, false), 1.0);
        assert_eq!(sheer(0.6, false), 0.6);
    }

    /// What the desktop loses behind a tube's pane at its thickest — the
    /// top of the frost, etched, at the High level — stays small. The first
    /// sheer try (smoke at .42 over a .45 page) covered 68% and still read
    /// as a dark window.
    #[test]
    fn a_tube_pane_lets_most_of_the_desktop_through() {
        let _g = crate::app::theme_test_guard();
        for id in crew_theme::ALL_THEMES {
            if !id.theme().is_tube() {
                continue;
            }
            let s = crew_theme::glass_style_for(id.theme()).scaled(crew_theme::GlassLevel::High);
            let a = (s.alpha_top + s.etch).min(1.0);
            let cover = a + TUBE_OPACITY * (1.0 - a);
            assert!(cover <= 0.3, "{}: a pane covers {cover:.2}", id.as_str());
        }
    }

    #[test]
    fn the_app_reads_the_tube_cap_from_the_active_theme() {
        let _g = crate::app::theme_test_guard();
        let app = CrewApp::default();
        crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
        assert!(app.window_opacity() <= TUBE_OPACITY);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        assert_eq!(app.window_opacity(), app.config.window_opacity);
    }
}
