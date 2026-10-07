use super::*;
use crate::chat::ChatPane;
use crate::chatswarmrows::tests::{diamond, pane_with, rows};
use crew_hive::{AgentId, HiveEvent, TaskId};

const WHY: &str = "api error: model qwen-maxx does not exist";

/// The second task's agent fails with the provider's words — a hint on the
/// next line, as a provider sends one — and then the task does.
fn failed() -> ChatPane {
    let mut p = pane_with(diamond());
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(4),
        task: TaskId(11),
    });
    p.absorb_hive(&HiveEvent::Failed {
        agent: AgentId(4),
        error: format!("{WHY}\nhint: /model lists what this host serves"),
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(11),
        state: TaskState::Failed,
    });
    p
}

#[test]
fn a_failed_row_says_why_on_the_line_under_it_clipped_to_the_pane() {
    let p = failed();
    let r = rows(&p, 100, 0);
    assert_eq!(r.len(), 4, "three rows and one reason: {r:#?}");
    assert!(r[1].starts_with(" 2 \u{2717} writer"), "{}", r[1]);
    assert_eq!(
        r[2],
        format!("     {WHY}"),
        "under the words, first line only"
    );
    assert!(r[3].starts_with(" 3 \u{25cb} critic"), "{}", r[3]);
    // Narrow: cut at a word, with the mark, inside the pane's margin.
    let r = rows(&p, 30, 0);
    assert_eq!(r[2], "     api error: model\u{2026}");
    for cols in [12u16, 30, 46, 60, 160] {
        let r = rows(&p, cols, 0);
        let w = crate::chatwidth::str_w(&r[2]);
        assert!(w < usize::from(cols), "cols={cols}: {w} wide: {:?}", r[2]);
    }
    assert_eq!(
        crate::chatswarmview::swarm_rows(&p, 100),
        5,
        "the pane budgets the reason's row"
    );
}

#[test]
fn only_a_failed_task_wears_a_reason_and_a_retry_starts_without_one() {
    let mut p = failed();
    let s = p.swarm.as_mut().unwrap();
    assert_eq!(why(&s.tasks[1]), Some(WHY));
    s.apply_at(
        &HiveEvent::AgentSpawned {
            agent: AgentId(5),
            task: TaskId(11),
        },
        0,
    );
    assert_eq!(why(&s.tasks[1]), None, "running again");
    assert!(
        s.tasks[1].why.is_empty(),
        "the old attempt's reason is gone"
    );
    // A failure nobody explained has no line to draw.
    s.apply_at(
        &HiveEvent::TaskStateChanged {
            task: TaskId(11),
            state: TaskState::Failed,
        },
        0,
    );
    assert_eq!(why(&s.tasks[1]), None);
    assert_eq!(crate::chatswarmplan::rows_wanted(s), 3);
}

#[test]
fn reasons_share_the_list_height_and_the_tail_counts_what_they_pushed_out() {
    use crate::chatswarmplan::{lines, shown, Line, MAX_ROWS};
    let tasks = (0..7)
        .map(|i| {
            let mut t = diamond().remove(0);
            t.id = TaskId(i);
            t
        })
        .collect();
    let mut p = pane_with(tasks);
    let s = p.swarm.as_mut().unwrap();
    for i in [0u64, 2] {
        s.apply_at(
            &HiveEvent::AgentSpawned {
                agent: AgentId(i),
                task: TaskId(i),
            },
            0,
        );
        s.apply_at(
            &HiveEvent::Failed {
                agent: AgentId(i),
                error: WHY.into(),
            },
            0,
        );
        s.apply_at(
            &HiveEvent::TaskStateChanged {
                task: TaskId(i),
                state: TaskState::Failed,
            },
            0,
        );
    }
    // 7 tasks + 2 reasons = 9 rows: five tasks, their two reasons, the tail.
    assert_eq!(shown(s), (5, 2));
    let l = lines(s);
    assert_eq!(l.len(), MAX_ROWS);
    assert_eq!(&l[..3], &[Line::Task(0), Line::Why(0), Line::Task(1)]);
    assert_eq!(l[MAX_ROWS - 1], Line::Tail(2));
}

#[test]
fn the_record_lists_the_failed_task_with_its_reason() {
    use crate::chatmsgs::{card_lines, View};
    let mut p = failed();
    for id in [10, 12] {
        p.absorb_hive(&HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state: TaskState::Done,
        });
    }
    assert!(p.swarm.is_none(), "folded");
    let text = p.messages[0].text.clone();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 5, "header, three rows, one reason: {lines:#?}");
    assert_eq!(
        lines[2],
        " 2 \u{2717} writer  draft the answer\u{a0}\u{2190}\u{a0}1"
    );
    assert_eq!(lines[3], format!(" {}{WHY}", "\u{a0}".repeat(4)));
    // The card draws it as its own line, under the words of the row it
    // explains — markdown strips leading spaces, and flush left it read as
    // a row of its own.
    p.messages[0].expanded = true;
    let drawn: Vec<String> = card_lines(&[&p.messages[0]], 100, 0, View::default())
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect::<String>())
        .collect();
    let col = |l: &str, word: &str| l.find(word).map(|b| l[..b].chars().count());
    let at = drawn.iter().position(|l| l.contains("writer")).unwrap();
    assert_eq!(
        col(&drawn[at + 1], WHY),
        col(&drawn[at], "writer"),
        "{drawn:#?}"
    );
    // A reason too long for one line is cut at a word, with the mark.
    let long = format!("{WHY} {}", "and then some more words ".repeat(8));
    let mut q = pane_with(diamond());
    let s = q.swarm.as_mut().unwrap();
    s.tasks[0].state = TaskState::Failed;
    s.tasks[0].why = long;
    let line = record_line(s, 0).unwrap();
    assert!(line.ends_with('\u{2026}'), "{line}");
    assert!(
        crate::chatwidth::str_w(line.trim_start()) <= RECORD_W,
        "{line}"
    );
}
