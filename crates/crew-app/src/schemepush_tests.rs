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
    // Same darkness again: quiet (a dark→dark rotation is not a change) —
    // and every palette is dark glass since 2026-10-09, light mode too.
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperLight),
        1,
    );
    assert!(!app.push_scheme_change());
    assert_eq!(app.scheme_pushed, Some(true));
}
