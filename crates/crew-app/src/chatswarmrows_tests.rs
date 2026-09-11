use super::*;
use crew_hive::{AgentId, AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec};
use crew_plugin::Plugin;

fn spec(id: u64, specialty: &str, title: &str, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: "p".into(),
        specialty: specialty.into(),
        expertise: String::new(),
    }
}

/// research → draft → review, the review also reading the research: three
/// tasks whose rows must show both edges.
pub(crate) fn diamond() -> Vec<TaskSpec> {
    vec![
        spec(10, "scout", "research the topic", &[]),
        spec(11, "writer", "draft the answer", &[10]),
        spec(12, "critic", "review the draft", &[10, 11]),
    ]
}

pub(crate) fn pane_with(tasks: Vec<TaskSpec>) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.absorb_hive_plan(tasks);
    p
}

fn apply(p: &mut ChatPane, ev: HiveEvent, at: u64) {
    p.swarm.as_mut().unwrap().apply_at(&ev, at);
}

pub(crate) fn state(p: &mut ChatPane, id: u64, state: TaskState, at: u64) {
    apply(
        p,
        HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state,
        },
        at,
    );
}

/// The text of one row, gaps as spaces, trailing space trimmed.
pub(crate) fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    let mut out = String::new();
    let mut x = 0u16;
    for c in v {
        while x < c.col {
            out.push(' ');
            x += 1;
        }
        out.push(c.c);
        x += crate::chatwidth::char_w(c.c) as u16;
    }
    out.trim_end().to_string()
}

pub(crate) fn rows(p: &ChatPane, cols: u16, now: u64) -> Vec<String> {
    let s = p.swarm.as_ref().unwrap();
    let cells = cells(p, cols, 0, now);
    (0..rows_wanted(s)).map(|r| row_text(&cells, r)).collect()
}

#[test]
fn every_task_gets_a_numbered_row_with_its_specialist_title_and_deps() {
    let p = pane_with(diamond());
    let r = rows(&p, 100, 0);
    assert_eq!(r[0], " 1 \u{25cb} scout  research the topic");
    assert_eq!(r[1], " 2 \u{25cb} writer  draft the answer \u{2190} 1");
    assert_eq!(r[2], " 3 \u{25cb} critic  review the draft \u{2190} 1,2");
    assert_eq!(
        rows_wanted(p.swarm.as_ref().unwrap()),
        3,
        "no tail for three"
    );
}

#[test]
fn the_glyphs_follow_the_states_as_hive_events_land() {
    let mut p = pane_with(diamond());
    apply(
        &mut p,
        HiveEvent::AgentSpawned {
            agent: AgentId(0),
            task: TaskId(10),
        },
        10,
    );
    let r = rows(&p, 100, 20);
    assert!(
        r[0].starts_with(" 1 \u{25cf} scout"),
        "spawned = running: {}",
        r[0]
    );
    state(&mut p, 10, TaskState::Done, 30);
    state(&mut p, 11, TaskState::Running, 30);
    state(&mut p, 11, TaskState::Failed, 40);
    state(&mut p, 12, TaskState::Cancelled, 40);
    let r = rows(&p, 100, 50);
    assert!(r[0].starts_with(" 1 \u{2713}"), "done: {}", r[0]);
    assert!(r[1].starts_with(" 2 \u{2717}"), "failed: {}", r[1]);
    assert!(r[2].starts_with(" 3 \u{2013}"), "cancelled: {}", r[2]);
}

#[test]
fn twelve_tasks_show_seven_rows_and_a_tail_naming_the_other_five() {
    let tasks: Vec<TaskSpec> = (0..12)
        .map(|i| spec(i, "", &format!("task {i}"), &[]))
        .collect();
    let p = pane_with(tasks);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(shown(12), (7, 5));
    assert_eq!(rows_wanted(s), 8, "seven rows and the tail");
    let r = rows(&p, 60, 0);
    assert_eq!(r.len(), 8);
    assert_eq!(
        r[6], "  7 \u{25cb} task 6",
        "the seventh row, numbered two wide"
    );
    assert_eq!(r[7], " \u{2026} +5 more tasks");
    assert_eq!(
        tail(1),
        "\u{2026} +1 more task",
        "one hidden task is singular"
    );
    assert_eq!(shown(8), (8, 0), "eight fit whole");
    assert_eq!(shown(9), (7, 2));
}
