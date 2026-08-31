use super::*;

/// A second window's panes are at stake too: Cmd+Q takes the app.
#[test]
fn panes_in_another_window_count() {
    let mut app = CrewApp {
        other_panes: 2,
        ..Default::default()
    };
    assert!(!app.confirm_quit(), "armed rather than quitting");
    let said = app
        .status
        .as_ref()
        .map(|(s, _)| s.clone())
        .unwrap_or_default();
    assert!(said.contains('2'), "said: {said:?}");
    assert!(app.confirm_quit(), "a second ask goes through");
}

#[test]
fn no_panes_exits_immediately() {
    assert!(quit_decision(false, None, Instant::now()));
}

#[test]
fn first_press_with_panes_does_not_exit() {
    assert!(!quit_decision(true, None, Instant::now()));
}

#[test]
fn second_press_within_window_exits() {
    let now = Instant::now();
    // armed just now → still within the confirmation window
    assert!(quit_decision(true, Some(now), now));
}

/// A stale arm must not still be live: walking away and clicking close an
/// hour later has to ask again, not exit on the first click.
#[test]
fn an_expired_arm_does_not_exit() {
    let now = Instant::now();
    let stale = now - QUIT_WINDOW - Duration::from_millis(1);
    assert!(!quit_decision(true, Some(stale), now));
}

fn app_with_a_pane() -> CrewApp {
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;
    let mut app = CrewApp::default();
    app.panes.push(Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: Some("pane".into()),
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    });
    app
}

/// The guard both the close button and Cmd+Q now share: with a pane open
/// the first request arms and explains, the second exits.
#[test]
fn a_live_pane_takes_two_requests_to_close() {
    let mut app = app_with_a_pane();
    assert!(!app.confirm_quit(), "first request must not exit");
    let msg = app.status.clone().map(|(m, _)| m).unwrap_or_default();
    assert!(msg.contains("1 pane open"), "no explanation given: {msg}");
    assert!(app.confirm_quit(), "second request should exit");
}

/// Nothing running → closing is immediate. The guard exists to protect live
/// work, not to make an empty window argue with you.
#[test]
fn an_empty_window_closes_on_the_first_request() {
    let mut app = CrewApp::default();
    assert!(app.confirm_quit());
    assert!(app.status.is_none(), "an empty window should not prompt");
}

/// The prompt answers a keypress AND a click, so it must not name either
/// input — "press quit again" is wrong when you clicked the close button.
#[test]
fn prompt_is_action_neutral_and_counts_panes() {
    let one = quit_prompt(1);
    assert!(one.contains("1 pane open"), "{one}");
    assert!(quit_prompt(3).contains("3 panes open"));
    for p in [one, quit_prompt(3)] {
        assert!(!p.contains("press"), "prompt names a keypress: {p}");
        assert!(!p.contains("click"), "prompt names a click: {p}");
    }
}
