//! The code line's row: granted while a turn of the pane's own runs, drawn
//! directly above the composer.
use super::*;
use crate::chat::tests::pane;

const COLS: u16 = 60;

/// The row as text, one char a column.
fn text(cells: &[CellView]) -> String {
    let mut row = vec![' '; COLS as usize];
    for c in cells {
        row[c.col as usize] = c.c;
    }
    row.into_iter().collect()
}

#[test]
fn the_line_has_a_row_only_while_a_turn_of_its_own_runs() {
    let mut p = pane();
    assert_eq!(code_rows(&p, COLS), 0, "an idle pane has nothing to say");
    p.awaiting = true;
    assert_eq!(
        code_rows(&p, COLS),
        1,
        "from the moment the message is sent"
    );
    assert_eq!(live_rows(&p, COLS), 1, "and it takes the bar's row");
    assert_eq!(code_rows(&p, MIN_COLS - 1), 0, "a pane too narrow drops it");
    let (id, question, since) = ("1".into(), "run it?".into(), 0);
    p.asking.push_back(crate::chatapprove::Asking {
        id,
        question,
        since,
    });
    assert_eq!(
        code_rows(&p, COLS),
        0,
        "a question waiting on you is not work"
    );
}

#[test]
fn a_busy_pane_draws_it_directly_above_the_composer() {
    let mut p = pane();
    p.absorb_delta("smith".into(), "hello".into(), false);
    let idle = crate::chatplace::grants(&p, COLS, 20);
    p.awaiting = true;
    let g = crate::chatplace::grants(&p, COLS, 20);
    assert_eq!(
        (g.prog, g.msg + 1),
        (1, idle.msg),
        "one row out of the transcript"
    );
    let row = 20 - g.bottom - g.plan - 1;
    let cells: Vec<CellView> = p
        .cells(COLS, 20)
        .into_iter()
        .filter(|c| c.row == row)
        .collect();
    // No poll has stamped the turn, so it began this frame.
    assert!(
        text(&cells).trim_end().ends_with("0s"),
        "{:?}",
        text(&cells)
    );
}

#[path = "chatcodelinetool_tests.rs"]
mod tool_tests;
