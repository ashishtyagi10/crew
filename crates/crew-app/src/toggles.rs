//! Palette-discoverable toggles that mirror the Cmd-chord shortcuts: `/broadcast`
//! (Cmd+S), `/zoom` (Cmd+Z), `/sidebar` (Cmd+G). The fuzzy command palette
//! surfaces them by name when you can't recall the chord. The chords call the
//! same methods, so behaviour stays in lockstep.
use crate::app::CrewApp;
use crate::chords::broadcast_label;

impl CrewApp {
    /// Toggle broadcast — mirror typed input to every terminal pane.
    pub(crate) fn toggle_broadcast(&mut self) {
        self.broadcast = !self.broadcast;
        self.input.broadcast = self.broadcast;
        self.set_status(broadcast_label(self.broadcast));
        self.redraw();
    }

    /// Advance the theme cycle (Ctrl+Shift+L): every fixed theme in order, then
    /// `random`, then wraps — so the one hotkey reaches all of them, persists
    /// the choice, and repaints exactly like the `/theme` command.
    pub(crate) fn toggle_theme(&mut self) {
        let label = crew_theme::cycle_next(crate::chattime::unix_now_ms());
        self.config.theme = Some(label.to_string());
        crate::palette::set_accent(self.config.accent_rgb());
        self.config.save();
        self.redraw();
        self.set_status(format!("theme: {label}"));
    }

    /// Toggle zoom — the focused pane fills the content area.
    pub(crate) fn toggle_zoom(&mut self) {
        self.zoomed = !self.zoomed;
        self.set_status(if self.zoomed { "zoomed" } else { "unzoomed" });
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn toggle_theme_cycles_forward() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        let mut app = crate::app::CrewApp::default();
        app.toggle_theme();
        assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
        app.toggle_theme();
        // Past the two paper themes it steps into the CRT set (no longer a flip).
        assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::CrtGreen);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }

    #[test]
    fn toggle_theme_enters_random_after_the_last_fixed_theme_then_wraps() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_random(false, 0);
        crew_theme::set_theme(crew_theme::ThemeId::CrtBlue);
        let mut app = crate::app::CrewApp::default();
        app.toggle_theme();
        assert!(crew_theme::is_random());
        assert_eq!(app.config.theme.as_deref(), Some("random"));
        app.toggle_theme();
        assert!(!crew_theme::is_random());
        assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperDark);
        assert_eq!(app.config.theme.as_deref(), Some("paper-dark"));
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }

    #[test]
    fn toggle_broadcast_flips_and_mirrors_input() {
        let mut app = CrewApp::default();
        assert!(!app.broadcast && !app.input.broadcast);
        app.toggle_broadcast();
        assert!(app.broadcast && app.input.broadcast);
        app.toggle_broadcast();
        assert!(!app.broadcast && !app.input.broadcast);
    }

    #[test]
    fn toggle_zoom_flips() {
        let mut app = CrewApp::default();
        app.toggle_zoom();
        assert!(app.zoomed);
        app.toggle_zoom();
        assert!(!app.zoomed);
    }
}
