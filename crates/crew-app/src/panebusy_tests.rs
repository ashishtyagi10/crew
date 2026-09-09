//! The two redraw registries a crew pane can land in: busy (`pane_animating`,
//! 15 fps) and ambient (`pane_breathing` → `CrewApp::ambient_breath`, 6 fps).
//! An idle connected pane's breathing dot must be the second, never the first.
use super::*;
use crate::app::CrewApp;
use crate::chat::ChatPane;
use crate::motion::{set_level, MotionLevel};
use crew_plugin::Plugin;

fn chat_pane(connected: bool) -> Pane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut chat = ChatPane::new(plugin, "crew".into());
    chat.connected = connected;
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Chat(chat),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: Some("crew".into()),
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

#[test]
fn an_idle_connected_crew_pane_breathes_ambiently_not_busily() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let mut app = CrewApp {
        win_focus: None,
        ..Default::default()
    };
    app.panes.push(chat_pane(true));
    assert!(
        !pane_animating(&app.panes[0]),
        "idle must NOT sit in the busy branch (15 fps)"
    );
    assert!(pane_breathing(&app.panes[0]));
    assert!(app.ambient_breath(), "…but does register ambiently (6 fps)");

    app.win_focus = Some(false);
    assert!(!app.ambient_breath(), "no repaint for a window nobody sees");
    app.win_focus = Some(true);
    assert!(app.ambient_breath());

    set_level(MotionLevel::Off);
    assert!(!app.ambient_breath(), "Off is static: no ambient frames");
    set_level(MotionLevel::Full);
}

#[test]
fn only_a_connected_visible_idle_pane_breathes() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    assert!(
        !pane_breathing(&chat_pane(false)),
        "connecting: the hollow dot is still"
    );
    let mut hidden = chat_pane(true);
    hidden.hidden = true;
    assert!(
        !pane_breathing(&hidden),
        "minimised to the nav: nothing to draw"
    );
    let mut busy = chat_pane(true);
    if let PaneContent::Chat(c) = &mut busy.content {
        c.awaiting = true;
    }
    assert!(pane_animating(&busy), "busy IS the busy branch");
    assert!(
        !pane_breathing(&busy),
        "and never the ambient one at the same time"
    );
}
