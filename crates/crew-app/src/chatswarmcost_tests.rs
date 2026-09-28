use super::*;
use crate::chat::ChatPane;
use crate::chatswarmrows::tests::{diamond, pane_with, row_text, rows};
use crew_hive::{AgentId, HiveEvent, TaskId, TaskState};

/// Spawn `agent` on `task`, credit it `input`/`output` tokens, settle it in
/// `state` — the path a real run's spend takes (`TokenDelta` names agents).
fn spend(p: &mut ChatPane, agent: u64, task: u64, (input, output): (u32, u32), state: TaskState) {
    let s = p.swarm.as_mut().unwrap();
    let a = AgentId(agent);
    s.apply_at(
        &HiveEvent::AgentSpawned {
            agent: a.clone(),
            task: TaskId(task),
        },
        0,
    );
    s.apply_at(
        &HiveEvent::TokenDelta {
            agent: a,
            input,
            output,
        },
        10,
    );
    s.apply_at(
        &HiveEvent::TaskStateChanged {
            task: TaskId(task),
            state,
        },
        20,
    );
}

#[test]
fn a_finished_row_ends_in_its_spend_when_there_is_room_and_drops_it_first_when_not() {
    let mut p = pane_with(diamond());
    spend(&mut p, 1, 10, (1234, 340), TaskState::Done);
    let r = rows(&p, 100, 30);
    assert!(r[0].ends_with("\u{2191}1.2k \u{2193}340"), "{}", r[0]);
    assert!(r[0].contains("research the topic"), "{}", r[0]);
    // 40 columns: every title whole, and the count gone to keep them so.
    let r = rows(&p, 40, 30);
    assert!(r[0].contains("research the topic"), "{}", r[0]);
    assert!(!r[0].contains('\u{2191}'), "the count goes first: {}", r[0]);
    assert!(r[2].contains("review the draft"), "{}", r[2]);
}

#[test]
fn only_a_row_that_stopped_says_what_it_spent() {
    let mut p = pane_with(diamond());
    spend(&mut p, 1, 10, (900, 45), TaskState::Done);
    spend(&mut p, 2, 11, (2_000, 10), TaskState::Running);
    let r = rows(&p, 100, 30);
    assert!(r[0].ends_with("\u{2191}900 \u{2193}45"), "{}", r[0]);
    assert!(!r[1].contains('\u{2191}'), "still running: {}", r[1]);
    assert!(!r[2].contains('\u{2191}'), "never ran: {}", r[2]);
    assert_eq!(
        words(1_500, 0),
        "\u{2191}1.5k \u{2193}0",
        "the status line's wording"
    );
}

#[test]
fn the_bars_share_a_column_left_of_the_counts() {
    let mut p = pane_with(diamond());
    spend(&mut p, 1, 10, (1234, 340), TaskState::Done);
    spend(&mut p, 2, 11, (88, 9), TaskState::Failed);
    let cells = crate::chatswarmrows::cells(&p, 100, 0, 30);
    let bar_start = |row: u16| {
        let mut on: Vec<u16> = cells
            .iter()
            .filter(|c| c.row == row && matches!(c.c, '\u{2501}' | '\u{2500}'))
            .map(|c| c.col)
            .collect();
        on.sort_unstable();
        (on[0], *on.last().unwrap())
    };
    assert_eq!(bar_start(0), bar_start(1), "{}", row_text(&cells, 1));
    let (_, bar_end) = bar_start(0);
    let count = "\u{2191}1.2k \u{2193}340";
    let at = 99 - crate::chatwidth::str_w(count) as u16;
    assert_eq!(bar_end + 3, at, "a two-column gap before the widest count");
}

#[test]
fn counts_and_reasons_stay_inside_the_pane_and_never_overlap_at_any_width() {
    let mut p = pane_with(diamond());
    spend(&mut p, 1, 10, (1234, 340), TaskState::Done);
    spend(&mut p, 2, 11, (88_000, 9_100), TaskState::Failed);
    let s = p.swarm.as_mut().unwrap();
    s.tasks[1].why = "timed out after 120s waiting on the provider".into();
    for cols in 0..=160u16 {
        let cells = crate::chatswarmrows::cells(&p, cols, 0, 30);
        for row in 0..4u16 {
            let mut spans: Vec<(u16, u16)> = cells
                .iter()
                .filter(|c| c.row == row)
                .map(|c| (c.col, c.col + crate::chatwidth::char_w(c.c) as u16))
                .collect();
            spans.sort_unstable();
            assert!(
                spans.last().is_none_or(|s| s.1 <= cols),
                "cols={cols} row={row}"
            );
            for w in spans.windows(2) {
                assert!(w[0].1 <= w[1].0, "cols={cols} row={row}: {:?}", w);
            }
        }
    }
}
