//! Tests for the config/theme plumbing in `spawn.rs`.
use crate::app::CrewApp;
use crate::config::CrewConfig;

#[test]
fn hydrated_env_hands_spawns_the_detection_path() {
    // Run panes must execute against the SAME PATH commands are detected
    // with (cmdcheck::effective_path) — a Dock-launched app's inherited PATH
    // misses ~/.local/bin and /opt/homebrew/bin, so `claude` would pass
    // detection yet fail to spawn.
    let env = crate::spawn::hydrated_env();
    let path = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone());
    assert_eq!(path, Some(crate::cmdcheck::effective_path()));
    assert!(!path.unwrap().is_empty());
}

#[test]
fn apply_config_adopts_values_without_a_renderer() {
    // `apply_config` pins a theme app-wide (spawn.rs: no `config.theme` →
    // `apply_selection(Fixed(theme_id()))`, which clears the random MODE), so
    // even a test that only cares about font_size/show_nav must take the guard
    // — without it this races `persist_theme_saves_the_live_mode_name`, which
    // then reads "paper-dark" instead of its own "random-light".
    let _g = crate::app::theme_test_guard();
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
    // Guarded for the same reason: `apply_config` mutates the global theme
    // even though this test is only about the font.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let mut cfg = app.config.clone();
    cfg.font_family = Some("Menlo".to_string());
    app.apply_config(cfg);
    assert!(!app.font_rotate.on, "explicit family pick stops rotation");
}

#[test]
fn pinning_a_family_says_it_stopped_rotation() {
    // It used to stop silently — and pinning your own font back is the natural
    // reaction to a rotated pick you dislike, so rotation died without a word
    // and the feature read as "/font random only works once".
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let mut cfg = app.config.clone();
    cfg.font_family = Some("Menlo".to_string());
    app.apply_config(cfg);
    let status = app.active_status().unwrap_or_default();
    assert!(status.contains("Menlo"), "{status}");
    assert!(status.contains("rotation off"), "{status}");
}

#[test]
fn a_config_apply_that_does_not_touch_the_family_says_nothing() {
    // Only a genuine pin should report; every Settings save re-applies the
    // config, and a status line that fires on each one is noise.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let cfg = CrewConfig {
        font_size: 19.0,
        ..app.config.clone()
    };
    app.apply_config(cfg);
    assert!(
        !app.active_status()
            .unwrap_or_default()
            .contains("rotation off"),
        "a font-size change is not a pin"
    );
    assert!(app.font_rotate.on, "…and must not stop rotation");
}

/// An unrelated config touch must not re-roll the theme.
///
/// `apply_selection(Mode(..))` re-picks a theme AND restarts the 10-minute
/// clock. `apply_config` ran it on every apply — and a Cmd+= zoom, every
/// Settings save and every `/theme` all route through `apply_settings` →
/// `apply_config`. So the theme re-rolled whenever config was touched for any
/// reason, and a rotation cycle could never actually complete. It also made
/// rotation LOOK alive while the font (which has no such path) sat still.
#[test]
fn apply_config_does_not_reroll_an_already_active_rotation() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Mode(crew_theme::RandomMode::Dark),
        1_000,
    );
    let picked = crew_theme::current_id();
    let mut app = CrewApp::default();

    // A config apply for an unrelated reason (here a font size), with the
    // same rotation mode already live.
    app.apply_config(CrewConfig {
        theme: Some("random-dark".into()),
        font_size: 19.0,
        ..CrewConfig::default()
    });

    assert!(crew_theme::is_random(), "still rotating");
    assert_eq!(
        crew_theme::current_id(),
        picked,
        "an unrelated config touch re-rolled the theme — the rotation's own \
         10-minute clock is the only thing that may change it"
    );
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
