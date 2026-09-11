use super::*;
use crate::chatmsgs::{card_lines, View};
use crate::chatswarmrows::tests::{diamond, pane_with, row_text};
use crew_hive::{AgentId, TaskId};

fn state(p: &mut ChatPane, id: u64, state: TaskState, at: u64) {
    let ev = HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state,
    };
    p.swarm.as_mut().unwrap().apply_at(&ev, at);
}

/// Every task settles: two done, one failed. Goes through `absorb_hive` for
/// the last one so the fold itself is under test.
fn finished() -> ChatPane {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    state(&mut p, 10, TaskState::Done, 4_000);
    state(&mut p, 11, TaskState::Running, 4_000);
    state(&mut p, 11, TaskState::Done, 9_000);
    state(&mut p, 12, TaskState::Running, 9_000);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(12),
        state: TaskState::Failed,
    });
    p
}

#[test]
fn a_finished_run_folds_into_one_record_card_with_the_count_and_every_row() {
    let p = finished();
    assert!(p.swarm.is_none(), "the live block is gone");
    assert!(!p.is_busy());
    assert_eq!(p.messages.len(), 1, "one record, not a card per task");
    let m = &p.messages[0];
    assert_eq!(m.sender, "agent smith");
    let lines: Vec<&str> = m.text.lines().collect();
    assert!(
        lines[0].starts_with("swarm \u{b7} 3 tasks \u{b7} 2 done \u{b7} 1 failed"),
        "{}",
        lines[0]
    );
    assert_eq!(lines.len(), 4, "header and three rows: {:?}", lines);
    assert_eq!(lines[1], " 1 \u{2713} scout  research the topic");
    assert_eq!(
        lines[3],
        " 3 \u{2717} critic  review the draft \u{2190} 1,2"
    );
}

#[test]
fn the_record_says_how_long_the_run_took() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 1_000);
    state(&mut p, 10, TaskState::Done, 13_400);
    state(&mut p, 11, TaskState::Cancelled, 13_400);
    state(&mut p, 12, TaskState::Cancelled, 13_400);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(
        text(s, 99_000).lines().next().unwrap(),
        "swarm \u{b7} 3 tasks \u{b7} 1 done \u{b7} 2 cancelled \u{b7} 12s"
    );
}

#[test]
fn the_record_folds_like_a_system_card_and_opens_to_every_row() {
    let mut p = finished();
    let folded = card_lines(&[&p.messages[0]], 80, 0, View::default());
    assert_eq!(folded.len(), 2, "header + first row only while folded");
    let first: String = folded[1].iter().map(|c| c.c).collect();
    assert!(first.contains("\u{2026} +3"), "three rows hidden: {first}");
    p.messages[0].expanded = true;
    let open = card_lines(&[&p.messages[0]], 80, 0, View::default());
    assert_eq!(open.len(), 5, "header, count line, three rows");
}

#[test]
fn stats_after_the_fold_puts_the_tool_pool_on_the_record_once() {
    let mut p = finished();
    p.note_swarm_tools(Some((5, 12)));
    let head = p.messages[0].text.lines().next().unwrap().to_string();
    assert!(head.ends_with(" \u{b7} tools 5/12"), "{head}");
    p.note_swarm_tools(Some((6, 12)));
    assert_eq!(
        p.messages[0].text.lines().next().unwrap(),
        head,
        "a record that has the figure keeps it"
    );
    assert_eq!(
        p.messages[0].text.lines().count(),
        4,
        "the rows are untouched"
    );
}

#[test]
fn a_run_that_drew_from_the_pool_records_it_without_waiting_for_stats() {
    let mut p = pane_with(diamond());
    p.absorb_hive(&HiveEvent::ToolBudget { used: 3, total: 12 });
    for id in [10, 11, 12] {
        p.absorb_hive(&HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state: TaskState::Done,
        });
    }
    let head = p.messages[0].text.lines().next().unwrap();
    assert!(head.ends_with(" \u{b7} tools 3/12"), "{head}");
}

#[test]
fn stats_during_the_run_lands_on_the_live_line() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    p.note_swarm_tools(Some((5, 12)));
    assert_eq!(p.swarm.as_ref().unwrap().tools, Some((5, 12)));
    let cells = crate::chatswarmview::block_cells(&p, 80, 0, 0);
    let line = row_text(&cells, 0);
    assert!(line.ends_with("(tools 5/12)"), "{line}");
}

#[test]
fn the_status_line_carries_the_pool_beside_the_tokens_and_only_when_known() {
    let mut p = pane_with(diamond());
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(10),
    });
    let line = |p: &ChatPane| row_text(&crate::chatswarmview::block_cells(p, 80, 0, 0), 0);
    assert!(!line(&p).contains("tools"), "{}", line(&p));
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(0),
        input: 1200,
        output: 300,
    });
    assert!(
        line(&p).ends_with("(\u{2191}1.2k \u{2193}300)"),
        "{}",
        line(&p)
    );
    p.absorb_hive(&HiveEvent::ToolBudget { used: 5, total: 12 });
    assert!(
        line(&p).ends_with("(\u{2191}1.2k \u{2193}300 \u{b7} tools 5/12)"),
        "{}",
        line(&p)
    );
}

#[test]
fn an_unsized_or_unknown_pool_says_nothing() {
    assert_eq!(tools_words(None), None);
    assert_eq!(tools_words(Some((0, 0))), None);
    assert_eq!(tools_words(Some((0, 4))).as_deref(), Some("tools 0/4"));
    let mut p = pane_with(diamond());
    p.note_swarm_tools(None);
    assert_eq!(p.swarm.as_ref().unwrap().tools, None);
}

#[test]
fn the_broker_dying_mid_run_still_leaves_the_record() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    p.fold_swarm();
    assert!(p.swarm.is_none());
    let head = p.messages[0].text.lines().next().unwrap();
    assert!(
        head.starts_with("swarm \u{b7} 3 tasks \u{b7} 0 done"),
        "{head}"
    );
    assert!(
        p.messages[0].text.contains(" 1 \u{25cf} scout"),
        "still running when it died"
    );
}
