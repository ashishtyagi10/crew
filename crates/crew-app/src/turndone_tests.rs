use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

fn chat_pane() -> Pane {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Chat(crate::chat::ChatPane::new(plugin, "crew".into())),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: Some("smith".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

fn app() -> CrewApp {
    let mut app = CrewApp::default();
    app.config.notify = true;
    app.config.notify_agent_done = true;
    app.config.notify_min_secs = 10;
    app.panes = vec![chat_pane(), chat_pane()];
    app.focused = 1;
    app
}

/// A long turn ending in a pane you are not in says so and marks the pane.
#[test]
fn a_long_turn_elsewhere_is_announced_and_marked() {
    let mut app = app();
    app.chat_turns_done(vec![(0, 75_000)]);
    let said = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(said.contains("a 1m15 turn finished in smith"), "{said}");
    assert!(
        app.panes[0].attention.is_some(),
        "the pane carries the marker"
    );
}

/// A turn you watched end, or a short one, needs no announcing.
#[test]
fn a_watched_or_short_turn_is_not_announced() {
    let mut app = app();
    app.win_focus = Some(true);
    app.chat_turns_done(vec![(1, 75_000), (0, 4_000)]);
    assert!(app.status.is_none(), "{:?}", app.status);
    // …but the pane you are in, while you are in another app, is announced.
    app.win_focus = Some(false);
    app.chat_turns_done(vec![(1, 75_000)]);
    assert!(app.status.is_some());
}

/// The pane's clock: a turn is timed from going busy to going idle.
#[test]
fn a_pane_times_its_turn() {
    let mut app = app();
    let PaneContent::Chat(c) = &mut app.panes[0].content else {
        unreachable!()
    };
    c.track_turn();
    c.awaiting = true;
    c.track_turn();
    std::thread::sleep(std::time::Duration::from_millis(30));
    c.awaiting = false;
    c.track_turn();
    let ms = c.watch.turn_done.take().expect("the turn ended");
    assert!(ms >= 25, "{ms}");
}
