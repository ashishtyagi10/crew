use crate::app::CrewApp;

#[test]
fn first_tick_latches_without_reporting_then_flips_report() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    let mut app = CrewApp::default();
    // First tick: latch only — startup must not spray reports.
    assert!(!app.push_scheme_change());
    assert_eq!(app.scheme_pushed, Some(true));
    // Same darkness again: quiet (a dark→dark rotation is not a change).
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::GlassNight),
        1,
    );
    assert!(!app.push_scheme_change());
    assert_eq!(app.scheme_pushed, Some(true));
    // Light mode is a white Ink Reader page again (2026-10-16): a change,
    // reported.
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperLight),
        2,
    );
    app.push_scheme_change(); // no pane listens, so nothing is written
    assert_eq!(app.scheme_pushed, Some(false));
}
