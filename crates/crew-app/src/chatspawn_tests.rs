use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

/// A stand-in pane carrying `label` — enough to exercise the `/smith`
/// single-instance guardrail without a real broker subprocess.
fn labeled_pane(label: Option<&str>) -> Pane {
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
        label: label.map(str::to_string),
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn smith_focuses_the_existing_crew_pane_instead_of_spawning_a_second_broker() {
    let mut app = CrewApp::default();
    // A couple of unrelated panes, then the crew pane at index 2.
    app.panes.push(labeled_pane(None));
    app.panes.push(labeled_pane(None));
    app.panes.push(labeled_pane(Some("crew")));
    app.focused = 0;
    app.input.focused = true;

    app.spawn_crew_pane();

    assert_eq!(app.panes.len(), 3, "no duplicate broker pane was spawned");
    assert_eq!(app.focused, 2, "focus moved to the existing crew pane");
    assert!(!app.input.focused, "focus left the input bar for the pane");
}

#[test]
fn a_minimized_crew_pane_is_the_guardrail_target_too() {
    // The guard matches by label regardless of hidden state; reconcile_grid
    // restores it on the next render because focus left the input bar.
    let mut app = CrewApp::default();
    app.panes.push(labeled_pane(Some("crew")));
    app.panes[0].hidden = true;
    app.focused = 0;
    app.input.focused = true;

    app.spawn_crew_pane();

    assert_eq!(app.panes.len(), 1, "still no second broker");
    assert_eq!(app.focused, 0);
    assert!(
        !app.input.focused,
        "focus off the input bar lets reconcile_grid restore the pane"
    );
}
