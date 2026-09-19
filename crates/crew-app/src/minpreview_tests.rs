use super::*;

use crate::chatlayout::Message;
use crate::pane::Pane;

fn pane_with(content: PaneContent) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content,
        grid: crew_term::GridSize { cols: 80, rows: 24 },
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
        born_ms: 0,
    }
}

fn msg(text: &str) -> Message {
    Message {
        sender: "smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

/// A TUI pads its columns with runs of spaces; a thumbnail has twenty of them
/// in total, so the padding has to go before the words do.
#[test]
fn whitespace_runs_close_up() {
    assert_eq!(
        tidy("  tests   3 passed\t\t1 failed  ").as_deref(),
        Some("tests 3 passed 1 failed")
    );
}

/// Control bytes are not glyphs — a stray one would be drawn as a box.
#[test]
fn control_characters_are_dropped() {
    assert_eq!(tidy("ok\u{7}\u{1b}done").as_deref(), Some("okdone"));
}

#[test]
fn a_blank_line_says_nothing() {
    assert_eq!(tidy("   \n\t "), None);
    assert_eq!(tidy(""), None);
}

/// A chat message is a paragraph; the row takes its first real line rather
/// than running the whole reply together.
#[test]
fn only_the_first_written_line_is_taken() {
    assert_eq!(
        tidy("\n\nthe fix is in sdf.rs\nand the note in the doc").as_deref(),
        Some("the fix is in sdf.rs")
    );
}

/// The middle starts clear of the marker and stops clear of the count, and
/// neither of those is allowed to be overdrawn (the count is the older
/// promise: it is what the card said before there was a preview at all).
#[test]
fn the_preview_sits_between_the_marker_and_the_count() {
    let _g = crate::app::theme_test_guard();
    let cells = cells("running tests", 20, 2);
    assert!(cells.iter().all(|c| c.col >= LEFT), "ran into the marker");
    let right = cells.iter().map(|c| c.col).max().unwrap();
    assert!(right < 20 - 3, "ran into the count: {right}");
}

/// A narrow thumbnail says nothing rather than a syllable and an ellipsis.
#[test]
fn a_thumbnail_with_no_room_stays_empty() {
    let _g = crate::app::theme_test_guard();
    let enough = MIN_ROOM as u16 + LEFT;
    for cols in 0..enough {
        assert!(
            cells("running tests", cols, 0).is_empty(),
            "{cols} columns drew something"
        );
    }
    assert!(!cells("running tests", enough, 0).is_empty());
}

/// Wide characters advance two columns: a CJK line must not stack two glyphs
/// on one cell, and must not spill past the room it was given.
#[test]
fn wide_characters_take_their_two_columns() {
    let _g = crate::app::theme_test_guard();
    let cells = cells("\u{6e2c}\u{8a66}\u{6e2c}\u{8a66} ok", 16, 0);
    let mut used: Vec<u16> = cells.iter().map(|c| c.col).collect();
    used.sort_unstable();
    let before = used.len();
    used.dedup();
    assert_eq!(used.len(), before, "two glyphs in one cell");
    assert!(used.iter().all(|&c| c < 16), "spilled past the card");
}

/// A pane crew draws itself answers from its own state, through
/// `paneglance` — the strip asks every kind, not just the two with a grid.
#[test]
fn a_drawn_pane_answers_from_its_own_state() {
    let content = PaneContent::Todo(crate::todopane::test_pane(Vec::new()));
    let want = crate::paneglance::of(&content);
    assert!(want.is_some(), "the todo list said nothing");
    assert_eq!(of(&pane_with(content)), want);
    // …and a settings form still says nothing: the form IS its state.
    let p = pane_with(PaneContent::Settings(
        crate::settingspane::SettingsPane::new(crate::config::CrewConfig::default(), Vec::new()),
    ));
    assert_eq!(of(&p), None);
}

/// A chat pane waiting on an agent says who and what, not the last thing
/// said — a pane that is working is the one you might restore.
#[test]
fn a_working_chat_names_the_agent_and_its_tool() {
    let mut c = crate::chat::tests::pane();
    c.messages.push(msg("older reply"));
    c.active.push(crate::chatflow::ActiveAgent {
        name: "scout".into(),
        from: "user".into(),
        since: std::time::Instant::now(),
        tool: Some("sys:run".into()),
    });
    let p = pane_with(PaneContent::Chat(c));
    assert_eq!(of(&p).as_deref(), Some("scout \u{b7} sys:run"));
}

/// Idle, it is the last thing said.
#[test]
fn an_idle_chat_shows_the_last_message() {
    let mut c = crate::chat::tests::pane();
    c.messages
        .push(msg("the ring's track vanishes on paper light"));
    let p = pane_with(PaneContent::Chat(c));
    assert_eq!(
        of(&p).as_deref(),
        Some("the ring's track vanishes on paper light")
    );
}
