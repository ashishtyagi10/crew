use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

fn far_pane(name: &str) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: Some(name.to_string()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn minimize_focused_pane_moves_focus_to_nearest_visible() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(far_pane(n));
    }
    app.focused = 1;
    app.input.focused = false;
    app.zoomed = true;
    app.minimize_pane(1);
    assert!(app.panes[1].hidden);
    assert_eq!(app.focused, 0, "nearest visible pane takes focus");
    assert!(!app.input.focused);
    assert!(!app.zoomed, "minimize leaves zoom");
}

#[test]
fn minimize_unfocused_pane_keeps_focus() {
    let mut app = CrewApp::default();
    for n in ["a", "b"] {
        app.panes.push(far_pane(n));
    }
    app.focused = 0;
    app.input.focused = false;
    app.minimize_pane(1);
    assert!(app.panes[1].hidden);
    assert_eq!(app.focused, 0);
}

#[test]
fn minimize_last_visible_pane_focuses_input_bar() {
    let mut app = CrewApp::default();
    app.panes.push(far_pane("solo"));
    app.focused = 0;
    app.input.focused = false;
    app.minimize_pane(0);
    assert!(app.panes[0].hidden);
    assert!(app.input.focused, "no visible pane left → input bar");
}

#[test]
fn minimize_shows_the_nav_when_hidden() {
    // The pane minimizes *into* the nav, so the nav must become visible.
    let mut app = CrewApp::default();
    app.panes.push(far_pane("a"));
    app.config.show_nav = false;
    app.minimize_pane(0);
    assert!(app.config.show_nav);
}

#[test]
fn minimize_out_of_range_is_a_noop() {
    let mut app = CrewApp::default();
    app.minimize_pane(3);
    assert!(app.panes.is_empty());
}

#[test]
fn close_others_keeps_the_focused_pane() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(far_pane(n));
    }
    app.focused = 1; // the "b" pane
    app.zoomed = true;
    // Asks first; the same command again is the answer.
    app.close_other_panes();
    assert_eq!(app.panes.len(), 3, "the first /only closed something");
    app.close_other_panes();
    assert_eq!(app.panes.len(), 1);
    assert_eq!(app.focused, 0);
    assert_eq!(app.panes[0].name.as_deref(), Some("b"));
    assert!(!app.zoomed);
}

#[test]
fn close_others_is_a_noop_with_one_pane() {
    let mut app = CrewApp::default();
    app.panes.push(far_pane("solo"));
    app.close_other_panes();
    assert_eq!(app.panes.len(), 1);
    assert_eq!(app.panes[0].name.as_deref(), Some("solo"));
}

/// The ask stood for ten seconds and was *visible* for three. It is a
/// state now, and the bar carries it for the whole window.
#[test]
fn the_question_reaches_the_bar_for_as_long_as_it_stands() {
    let mut app = CrewApp::default();
    app.panes.push(far_pane("a"));
    app.panes.push(far_pane("b"));
    app.close_all_panes();
    assert_eq!(app.panes.len(), 2, "the first run only asks");
    let q = app
        .pending
        .question(std::time::Instant::now())
        .expect("the question stands");
    assert!(q.contains("close all 2 panes"), "{q:?}");
    app.close_all_panes();
    assert!(app.panes.is_empty(), "the second run answers it");
    assert!(
        app.pending.question(std::time::Instant::now()).is_none(),
        "and the bar stops saying it"
    );
}

/// A confirmation you have moved on from is not still armed. `/closeall`
/// asks, you go and change the gradient instead, and a second `/closeall`
/// used to fire on the first press.
#[test]
fn an_unrelated_command_disarms_a_pending_confirmation() {
    let mut app = CrewApp::default();
    app.panes.push(far_pane("a"));
    app.panes.push(far_pane("b"));
    app.run_slash_command("closeall");
    assert!(app.pending.armed(), "the first run asks");
    app.run_slash_command("gradient");
    assert!(!app.pending.armed(), "going elsewhere disarmed it");
    app.run_slash_command("closeall");
    assert_eq!(app.panes.len(), 2, "so it has to ask again");
}
