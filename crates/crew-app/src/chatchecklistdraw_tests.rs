//! The plan's rows over the code line: the marks, the weights, the tally, and
//! where they sit in the live region.
use crew_hive::tools::todowire::{Step, StepState};
use crew_render::CellView;

use super::{live_cells, plan_cells};
use crate::chat::tests::pane;

const COLS: u16 = 60;

fn steps() -> Vec<Step> {
    let s = |text: &str, state| Step {
        text: text.into(),
        state,
    };
    vec![
        s("read the failing test", StepState::Done),
        s("find the root cause", StepState::Active),
        s("fix it and re-run", StepState::Pending),
    ]
}

/// Row `y` as text, one char a column.
fn text(cells: &[CellView], y: u16) -> String {
    let mut row = vec![' '; COLS as usize];
    for c in cells.iter().filter(|c| c.row == y) {
        row[c.col as usize] = c.c;
    }
    row.into_iter().collect::<String>().trim_end().to_string()
}

fn first<'a>(cells: &'a [CellView], y: u16, word: &str) -> &'a CellView {
    let line = text(cells, y);
    let at = line
        .find(word)
        .unwrap_or_else(|| panic!("{word:?} not in {line:?}"));
    let col = line[..at].chars().count() as u16;
    cells.iter().find(|c| c.row == y && c.col == col).unwrap()
}

#[test]
fn each_step_wears_the_swarm_rows_mark_for_where_it_is() {
    let _g = crate::app::theme_test_guard();
    let cells = plan_cells(&steps(), COLS, 0, 3);
    assert!(text(&cells, 0).starts_with(" \u{2713} read the failing test"));
    assert!(text(&cells, 1).starts_with(" \u{25cf} find the root cause"));
    assert!(text(&cells, 2).starts_with(" \u{25cb} fix it and re-run"));
}

#[test]
fn the_step_it_is_on_is_bold_and_what_is_done_recedes() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let cells = plan_cells(&steps(), COLS, 0, 3);
    let done = first(&cells, 0, "read");
    let on = first(&cells, 1, "find");
    let left = first(&cells, 2, "fix");
    assert_eq!((on.fg, on.bold), (t.ink, true));
    assert_eq!((left.fg, left.bold), (t.ink, false));
    assert_eq!((done.fg, done.bold), (t.text_muted, false));
}

#[test]
fn the_tally_sits_at_the_first_rows_edge() {
    let _g = crate::app::theme_test_guard();
    let cells = plan_cells(&steps(), COLS, 0, 3);
    assert!(text(&cells, 0).ends_with("1/3"), "{:?}", text(&cells, 0));
    assert_eq!(cells.iter().map(|c| c.col).max(), Some(COLS - 2));
    assert!(!text(&cells, 1).contains('/'));
}

#[test]
fn a_narrow_pane_gives_the_steps_the_width_and_never_spills() {
    let _g = crate::app::theme_test_guard();
    let cells = plan_cells(&steps(), 14, 0, 3);
    assert!(!text(&cells, 0).contains("1/3"));
    assert!(cells.iter().all(|c| c.col < 13));
}

#[test]
fn the_code_line_keeps_the_last_row_and_the_plan_stacks_over_it() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.awaiting = true;
    p.checklist = steps();
    assert_eq!(
        crate::chatchecklist::rows(&p, COLS),
        5,
        "3 steps, air, line"
    );
    let cells = live_cells(&p, COLS, 9, 5);
    assert!(
        cells.iter().all(|c| c.row != 9),
        "row 9 is the air over the plan"
    );
    assert!(text(&cells, 10).contains("read the failing test"));
    assert!(text(&cells, 12).contains("fix it and re-run"));
    // Two rows short, the air goes, then the first step: the line holds.
    let short = live_cells(&p, COLS, 11, 3);
    assert!(
        text(&short, 11).contains("find the root cause"),
        "{:?}",
        text(&short, 11)
    );
    assert!(text(&short, 12).contains("fix it and re-run"));
    // Row 13 is the code line: its clock at the edge, no step on it. (Two
    // frames are not compared: the glyphs flick every 90 ms.)
    let is_line = |row: String| row.ends_with('s') && !row.contains("re-run");
    assert!(is_line(text(&cells, 13)), "{:?}", text(&cells, 13));
    // Granted only one row, the region is the code line alone.
    let one = live_cells(&p, COLS, 13, 1);
    assert!(is_line(text(&one, 13)));
    assert!(one.iter().all(|c| c.row == 13));
}

#[test]
fn an_idle_pane_draws_no_plan_and_wants_no_rows() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.checklist = steps();
    assert_eq!(crate::chatchecklist::rows(&p, COLS), 0);
    assert!(live_cells(&p, COLS, 10, 4).is_empty());
}

#[test]
fn the_plan_arrives_by_event_and_goes_when_the_run_ends() {
    let mut p = pane();
    p.absorb_task(7, true);
    p.checklist = steps();
    p.absorb_task(7, false);
    assert!(p.checklist.is_empty());
}
