//! A CRT tube is a terminal running in glass, so its window is never a black
//! slab (2026-10-06, the user: "we don't need background swirl in the crt
//! mode, just the glass and borders, it should be almost frosted glass, no
//! black, like transparent").
//!
//! Under a tube theme the window is always SHEER — the desktop frosted behind
//! it by the window server's blur (`titlebar::apply_window`) — whatever the
//! Opacity % setting says; a sheerer setting still wins. Everything that
//! reads the window's opacity reads [`CrewApp::window_opacity`], so the page,
//! the blur, the title bar, the solid overlays and the lifted borders all
//! agree on it.
use crate::app::CrewApp;

/// How opaque a tube's page is at most: the desktop shows plainly through
/// the gaps between the panes, and the panes' own smoked glass (crew-theme's
/// `tube_glass`) is what holds the text off it.
pub(crate) const TUBE_OPACITY: f32 = 0.45;

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
    fn a_tube_is_always_sheer_and_a_sheerer_setting_still_wins() {
        assert_eq!(sheer(1.0, true), TUBE_OPACITY);
        assert_eq!(sheer(0.35, true), 0.35);
        assert_eq!(sheer(1.0, false), 1.0);
        assert_eq!(sheer(0.6, false), 0.6);
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
