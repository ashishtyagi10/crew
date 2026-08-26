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

    /// Advance the theme cycle (Ctrl+Shift+L): `dark` → `light` → `crt`,
    /// wrapping — so the one hotkey reaches all three consolidated themes,
    /// persists the choice, and repaints exactly like the `/theme` command.
    pub(crate) fn toggle_theme(&mut self) {
        let label = crew_theme::cycle_next(crate::chattime::unix_now_ms());
        self.config.theme = Some(label.to_string());
        crate::palette::set_accent(self.config.accent_rgb());
        self.config.save();
        self.redraw();
        self.set_status(format!("theme: {label}"));
    }

    /// Toggle zoom — the focused pane fills the content area.
    /// Focus mode on/off (`/focus`). Entering resets the held count; leaving
    /// reports it as one card, so the mode's cost is bounded by a single
    /// line rather than by whatever you failed to notice.
    pub(crate) fn toggle_focus_mode(&mut self) {
        let on = !crate::focusmode::on();
        crate::focusmode::set(on);
        if on {
            self.held = crate::focusmode::Held::default();
            self.set_status("focus mode on \u{2014} nothing will interrupt (/focus to leave)");
        } else {
            let summary = std::mem::take(&mut self.held).summary();
            match summary {
                Some(line) => {
                    self.toasts
                        .push(line.clone(), "held", false, crate::anim::now_ms());
                    self.set_status(line);
                }
                None => self.set_status("focus mode off"),
            }
        }
        self.redraw();
    }

    pub(crate) fn toggle_zoom(&mut self) {
        // Remember where the pane was so the zoom can travel out of (and back
        // into) its own tile rather than cutting.
        self.zoom_from = self.panes.get(self.focused).map(|p| p.rect);
        self.zoom_anim =
            crate::ease::Timeline::start(crate::anim::now_ms(), 240, crate::motion::level());
        self.zoomed = !self.zoomed;
        self.set_status(if self.zoomed { "zoomed" } else { "unzoomed" });
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn toggle_theme_cycles_every_mode_and_wraps() {
        let _g = crate::app::theme_test_guard();
        // From a pinned palette the first press enters the dark rotation.
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
            0,
        );
        let mut app = crate::app::CrewApp::default();
        app.toggle_theme();
        assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Dark));
        assert_eq!(app.config.theme.as_deref(), Some("dark"));
        app.toggle_theme();
        assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Light));
        assert_eq!(app.config.theme.as_deref(), Some("light"));
        app.toggle_theme();
        assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Crt));
        assert_eq!(app.config.theme.as_deref(), Some("crt"));
        // Then the OS-following auto — four stops, no more: the modern glow
        // palettes are members of the dark and light pools, not two extra
        // presses on the way round.
        app.toggle_theme();
        assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Auto));
        assert_eq!(app.config.theme.as_deref(), Some("auto"));
        // ...and wraps back to dark.
        app.toggle_theme();
        assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Dark));
        assert_eq!(app.config.theme.as_deref(), Some("dark"));
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
            0,
        );
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

    /// Focus mode is a MODE, and every mode owes two things: it has to be
    /// visibly on, and leaving it has to account for what it did while it was.
    #[test]
    fn focus_mode_holds_notifications_and_reports_them_on_the_way_out() {
        let _g = crate::app::motion_test_guard();
        let mut app = CrewApp::default();
        assert!(!crate::focusmode::on());

        app.toggle_focus_mode();
        assert!(crate::focusmode::on());
        // Two errors while focused: both write the LOG, neither pops.
        app.set_status_level(crate::applog::LogLevel::Error, "boom");
        app.set_status_level(crate::applog::LogLevel::Error, "bang");
        assert_eq!(app.toasts.len(), 0, "focus mode must not pop cards");
        assert_eq!(app.held.toasts, 2, "…but it must count them");
        assert!(
            app.log.iter().filter(|e| e.text.contains("boom")).count() == 1,
            "held is not dropped: the LOG still has the line"
        );

        app.toggle_focus_mode();
        assert!(!crate::focusmode::on());
        assert_eq!(app.held.toasts, 0, "the count resets on the way out");
        assert_eq!(app.toasts.len(), 1, "one summary card");

        // Entering again starts from zero rather than resuming an old tally.
        app.toggle_focus_mode();
        assert_eq!(app.held.toasts, 0);
        app.toggle_focus_mode();
        assert_eq!(app.toasts.len(), 1, "nothing held, so no summary card");
        crate::focusmode::set(false);
    }
}
