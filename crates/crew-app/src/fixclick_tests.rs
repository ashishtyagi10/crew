//! The click on a failed command's mark, and the keys that reach the same
//! place, on the rig of [`super::tests`].
use super::tests::{app_after, ran, CELL, OUT};
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// The pixel centre of the border cell on content row `r` of pane 0.
pub(super) fn border_at(app: &CrewApp, r: u16) -> (f32, f32) {
    let rect = app.pane_hit_rects()[0].1;
    let dy = crate::panefit::slack(rect, CELL.0, CELL.1).1 / CELL.1;
    (
        rect.x + CELL.0 / 2.0,
        rect.y + (crate::cardmarks::row_y(r, dy) + 0.5) * CELL.1,
    )
}

#[test]
fn a_click_on_the_failed_mark_finds_the_pane_and_no_other_row_does() {
    let mut app = app_after("cargo test", OUT, 101);
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    let rows = crate::failmark::rows(t, app.panes[0].grid.rows);
    assert_eq!(rows.len(), 1, "one failed block is on screen: {rows:?}");
    let failed = rows[0];
    app.cursor = border_at(&app, failed);
    assert_eq!(app.failed_mark_at_cursor(), Some((0, failed)));
    for r in (0..app.panes[0].grid.rows).filter(|r| *r != failed) {
        app.cursor = border_at(&app, r);
        assert_eq!(app.failed_mark_at_cursor(), None, "row {r}");
    }
}

#[test]
fn a_click_on_a_successful_commands_row_does_nothing() {
    let mut app = app_after("cargo test", OUT, 0);
    for r in 0..app.panes[0].grid.rows {
        app.cursor = border_at(&app, r);
        assert_eq!(app.failed_mark_at_cursor(), None, "row {r}");
    }
}

/// The visible row of the failed mark whose command was `line`.
fn row_of(app: &CrewApp, line: &str) -> u16 {
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    let found = crate::failmark::in_view(t, app.panes[0].grid.rows)
        .into_iter()
        .find(|(_, s)| s.line.as_deref() == Some(line));
    found.expect("that failure is on screen").0
}

#[test]
fn an_older_mark_hands_over_its_own_command_not_the_last() {
    let mut app = app_after("cargo test one", "error one\r\n", 101);
    ran(&mut app, 3, ("cargo test two", "error two\r\n", 2));
    let one = row_of(&app, "cargo test one");
    app.cursor = border_at(&app, one);
    let (pane, row) = app.failed_mark_at_cursor().expect("on the first mark");
    let f = app.failure_at(pane, Some(row)).expect("the first failure");
    assert_eq!((f.command.as_str(), f.exit), ("cargo test one", 101));
    assert!(
        f.output.contains("error one") && !f.output.contains("error two"),
        "{f:?}"
    );
    let last = app.failure_of(0).expect("the second is the last");
    assert_eq!(last.command, "cargo test two");
}

#[test]
fn a_failed_mark_still_answers_after_a_later_success() {
    let mut app = app_after("cargo test one", "error one\r\n", 101);
    ran(&mut app, 3, ("ls", "a b\r\n", 0));
    assert_eq!(app.failure_of(0), None, "the last command succeeded");
    let one = row_of(&app, "cargo test one");
    let f = app.failure_at(0, Some(one)).expect("the older failure");
    assert_eq!(f.command, "cargo test one");
}

#[test]
fn the_chord_routes_to_fix_with_smith() {
    // A pane that did not fail: nothing is spawned, and the status says why.
    let mut app = app_after("cargo test", OUT, 0);
    app.handle_super_chord(";");
    let said = app.status.as_ref().map(|s| s.0.clone()).unwrap_or_default();
    assert!(said.starts_with("nothing failed here"), "{said}");
    assert_eq!(app.panes.len(), 1);
}

#[test]
fn the_hint_shows_once_a_failed_pane_is_back_at_its_prompt_in_view() {
    let app = app_after("cargo test", OUT, 101);
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    let hint = crate::failhint::seen(t, true).expect("a failure in view");
    assert!(hint.starts_with("failed (exit 101)"), "{hint}");
    assert_eq!(
        crate::failhint::seen(t, false),
        None,
        "not the focused pane"
    );
    let ok = app_after("cargo test", OUT, 0);
    let PaneContent::Terminal(t) = &ok.panes[0].content else {
        unreachable!()
    };
    assert_eq!(crate::failhint::seen(t, true), None, "it passed");
}

#[test]
fn pressing_an_older_mark_prefills_that_command_into_smith() {
    let mut app = app_after("cargo test one", "error one\r\n", 101);
    ran(&mut app, 3, ("cargo test two", "error two\r\n", 2));
    // An open smith pane: the press focuses it instead of spawning a broker.
    let plugin = crew_plugin::Plugin::spawn("cat", &[]).unwrap();
    let mut smith = super::tests::pane();
    smith.content = PaneContent::Chat(crate::chat::ChatPane::new(plugin, "crew".into()));
    smith.label = Some("crew".into());
    app.panes.push(smith);
    app.reconcile_grid();
    app.build_frame();
    let one = row_of(&app, "cargo test one");
    app.cursor = border_at(&app, one);
    assert!(app.fix_mark_press(), "the press was the mark's");
    let PaneContent::Chat(c) = &app.panes[1].content else {
        unreachable!()
    };
    assert!(
        c.input.starts_with("`cargo test one` failed (exit 101)"),
        "{}",
        c.input
    );
    assert!(!c.input.contains("two"), "{}", c.input);
    assert_eq!(app.focused, 1, "smith has the focus");
}

#[test]
fn a_press_off_the_marks_is_not_the_marks() {
    let mut app = app_after("cargo test", OUT, 101);
    app.cursor = (500.0, 400.0);
    assert!(!app.fix_mark_press());
}
