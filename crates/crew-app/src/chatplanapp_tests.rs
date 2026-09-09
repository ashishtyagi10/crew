//! The app-level half of the plan button tests: the wire, and the
//! CrewApp hover/pointer plumbing. Split from `chatplanclick_tests` for
//! the line cap.
use super::*;
use std::ops::Range;

const COLS: u16 = 60;
const ROWS: u16 = 20;

/// A pane with a plan pending, and the row its buttons sit on.
fn pending() -> (ChatPane, u16) {
    let mut p = crate::chat::tests::pane();
    p.plan_pending = true;
    let row = p
        .plan_row(COLS, ROWS)
        .expect("a pending plan has a button row");
    (p, row)
}

fn span(btn: Btn) -> Range<u16> {
    buttons(COLS, crate::glyphs::on(), None, None)
        .unwrap()
        .spans
        .into_iter()
        .find(|(b, _)| *b == btn)
        .unwrap()
        .1
}

fn click(p: &mut ChatPane, row: u16, col: u16) -> Option<&'static str> {
    assert!(
        p.plan_press_at(COLS, ROWS, row, col),
        "the press lands on a button"
    );
    p.plan_release_at(COLS, ROWS, Some((row, col)), false)
}

#[test]
fn the_word_reaches_the_broker_on_the_wire() {
    let _g = crate::app::motion_test_guard();
    // A broker stand-in that records its stdin, so the test reads what the
    // click actually sent rather than what the pane says it sent.
    let dir = std::env::temp_dir().join(format!("crew-planbtn-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("stdin.jsonl");
    let cmd = format!("cat > '{}'", file.display());
    let plugin = crew_plugin::Plugin::spawn("sh", &["-c".to_string(), cmd]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.plan_pending = true;
    let row = p.plan_row(COLS, ROWS).unwrap();
    assert_eq!(
        click(&mut p, row, span(Btn::Run).start + 1),
        Some("approve")
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut got = String::new();
    while std::time::Instant::now() < deadline {
        got = std::fs::read_to_string(&file).unwrap_or_default();
        if got.contains("approve") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        got.contains("\"text\":\"approve\""),
        "sent to the broker: {got:?}"
    );
}

#[test]
fn the_hovered_button_draws_lifted_and_the_pointer_becomes_the_hand() {
    let _g = crate::app::theme_test_guard();
    let _plain = crate::glyphs::force(false);
    let (mut p, row) = pending();
    let rest = crate::chatview::cells(&p, COLS, ROWS);
    p.plan_hover_at(COLS, ROWS, Some((row, span(Btn::Run).start + 1)));
    let hover = crate::chatview::cells(&p, COLS, ROWS);
    let mark =
        |cells: &[crew_render::CellView]| cells.iter().find(|c| c.c == '\u{25b6}').unwrap().bg;
    assert_ne!(mark(&rest), mark(&hover), "the frame shows the lift");
    let mut app = crate::app::CrewApp::default();
    app.panes.push(crate::pane::Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Chat(p),
        grid: crew_term::GridSize {
            cols: COLS,
            rows: ROWS,
        },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    });
    assert!(app.plan_hover_on(0), "the pane reports its hovered button");
    assert_eq!(
        crate::pointer::icon(crate::pointer::Over::Link),
        winit::window::CursorIcon::Pointer,
        "which wears the hand a link gets"
    );
    // No window: the cursor is nowhere, so a sync clears it (a repaint).
    assert!(app.plan_hover_sync());
    assert!(!app.plan_hover_on(0));
    assert!(!app.plan_hover_sync(), "and a second sync changes nothing");
}
