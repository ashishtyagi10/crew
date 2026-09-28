//! The gap's two halves alone: which task each failure belongs to, and the
//! brief section that names them.
use super::run_tests::three;
use super::*;
use crew_hive::AgentId;

#[test]
fn a_clean_run_has_no_brief_section_at_all() {
    assert_eq!(Gap::default().brief(), "");
    assert!(Reasons::default()
        .gap(&TaskGraph::new(three()).unwrap(), &[])
        .is_empty());
}

#[test]
fn reasons_follow_the_agent_to_its_task_and_keep_the_first_error() {
    let graph = TaskGraph::new(three()).unwrap();
    let mut r = Reasons::default();
    let spawned = |a: u64, t: u64| HiveEvent::AgentSpawned {
        agent: AgentId(a),
        task: TaskId(t),
    };
    let failed = |a: u64, e: &str| HiveEvent::Failed {
        agent: AgentId(a),
        error: e.into(),
    };
    r.observe(&spawned(7, 2));
    r.observe(&failed(7, "provider timeout"));
    r.observe(&failed(7, "a later echo"));
    r.observe(&failed(9, "an agent never seen spawning"));
    // Task 1 failed with nothing said on the bus (a panicked agent); task 5
    // was re-planned in, so the plan never titled it.
    let brief = r.gap(&graph, &[TaskId(1), TaskId(2), TaskId(5)]).brief();
    assert!(
        brief.contains("\"Write the fix\", \"Run tests\" (provider timeout), \"task 5\"."),
        "{brief}"
    );
    assert!(
        !brief.contains("echo") && !brief.contains("never seen"),
        "{brief}"
    );
}

#[test]
fn a_rambling_reason_is_one_clipped_clause() {
    let graph = TaskGraph::new(three()).unwrap();
    let mut r = Reasons::default();
    r.observe(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(2),
    });
    r.observe(&HiveEvent::Failed {
        agent: AgentId(0),
        error: format!("line one\n\n{}", "x".repeat(400)),
    });
    let brief = r.gap(&graph, &[TaskId(2)]).brief();
    assert!(
        brief.contains("(line one xxx"),
        "whitespace flattened: {brief}"
    );
    assert!(brief.contains("x\u{2026})"), "clipped: {brief}");
    assert!(brief.len() < 400, "{brief}");
}
