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
