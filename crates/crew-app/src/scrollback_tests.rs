//! The `scrollback` setting: it survives a save and a load, is held in
//! range, reaches a pane at spawn, and reaches open panes when it changes.
use crate::app::CrewApp;
use crate::config::CrewConfig;
use crate::pane::PaneContent;
use crew_term::TermModel;

/// Lines above the screen in pane `i`.
fn history(app: &CrewApp, i: usize) -> usize {
    match &app.panes[i].content {
        PaneContent::Terminal(t) => t.pty.history_lines(),
        _ => unreachable!("a terminal pane"),
    }
}

/// Feed pane `i` far more lines than any scrollback here keeps.
fn flood(app: &mut CrewApp, i: usize) {
    if let PaneContent::Terminal(t) = &mut app.panes[i].content {
        t.pty.feed("x\r\n".repeat(3000).as_bytes());
    }
}

#[test]
fn scrollback_round_trips_and_is_clamped() {
    assert_eq!(CrewConfig::default().scrollback, 10_000);
    let cfg = CrewConfig {
        scrollback: 2500,
        ..CrewConfig::default()
    };
    let back = CrewConfig::from_toml_str(&cfg.to_toml_str());
    assert_eq!(back.scrollback, 2500, "carried through `clamped()`");
    assert_eq!(CrewConfig::from_toml_str("scrollback = 5").scrollback, 100);
    assert_eq!(
        CrewConfig::from_toml_str("scrollback = 9000000").scrollback,
        100_000
    );
}

/// A pane opened under `scrollback = N` keeps N lines, not alacritty's
/// 10 000; and a Settings save shrinks the panes already open.
#[test]
fn a_spawned_pane_keeps_the_setting_and_a_change_shrinks_it() {
    // `apply_config` sets the theme app-wide.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.config.scrollback = 1000;
    app.spawn_new_pane();
    assert_eq!(app.panes.len(), 1, "a shell opened");
    flood(&mut app, 0);
    assert_eq!(history(&app, 0), 1000, "spawned with the setting");

    let mut cfg = app.config.clone();
    cfg.scrollback = 200;
    app.apply_config(cfg);
    assert_eq!(history(&app, 0), 200, "the open pane shrinks at once");
    flood(&mut app, 0);
    assert_eq!(history(&app, 0), 200, "and stays at the new length");
}
