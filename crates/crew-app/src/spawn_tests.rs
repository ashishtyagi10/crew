//! Tests for the config/theme plumbing in `spawn.rs`.
use crate::app::CrewApp;
use crate::config::CrewConfig;

#[test]
fn apply_config_adopts_values_without_a_renderer() {
    let mut app = CrewApp::default();
    let cfg = CrewConfig {
        font_size: 19.0,
        show_nav: false,
        ..CrewConfig::default()
    };
    // No renderer in tests: the font calls are skipped, but config is adopted
    // and a relayout/redraw is safe to request.
    app.apply_config(cfg);
    assert_eq!(app.config.font_size, 19.0);
    assert!(!app.config.show_nav);
}

#[test]
fn manual_family_change_disables_rotation() {
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let mut cfg = app.config.clone();
    cfg.font_family = Some("Menlo".to_string());
    app.apply_config(cfg);
    assert!(!app.font_rotate.on, "explicit family pick stops rotation");
}

#[test]
fn apply_config_reconciles_random_mode() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    let mut app = CrewApp::default();

    // A saved `random` pin resumes rotation mode.
    app.apply_config(CrewConfig {
        theme: Some("random".into()),
        ..CrewConfig::default()
    });
    assert!(
        crew_theme::is_random(),
        "a saved `random` theme must resume rotation on apply"
    );

    // Applying a fixed theme (e.g. via the Settings pane) stops rotation and pins it.
    app.apply_config(CrewConfig {
        theme: Some("crt-green".into()),
        ..CrewConfig::default()
    });
    assert!(
        !crew_theme::is_random(),
        "picking a fixed theme in Settings must stop rotation"
    );
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::CrtGreen);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn set_theme_cmd_switches_active_theme() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = CrewApp::default();
    app.set_theme_cmd("paper-light");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    assert_eq!(app.config.theme.as_deref(), Some("paper-light"));
    // Unknown name leaves the active theme unchanged.
    app.set_theme_cmd("chartreuse");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

#[test]
fn set_theme_cmd_random_enters_rotation_mode() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    let mut app = CrewApp::default();
    app.set_theme_cmd("random");
    assert!(crew_theme::is_random());
    assert_eq!(app.config.theme.as_deref(), Some("random-dark"));

    // Switching to a fixed theme through this path also turns rotation off.
    app.set_theme_cmd("paper-light");
    assert!(!crew_theme::is_random());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}
