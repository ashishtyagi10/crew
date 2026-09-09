use super::*;
use crew_hive::AgentId;

fn call(agent: u64, label: &str, args: &str) -> HiveEvent {
    HiveEvent::ToolCall {
        agent: AgentId(agent),
        label: label.into(),
        args: args.into(),
    }
}

fn result(agent: u64, label: &str, ok: bool, ms: u64, text: &str) -> HiveEvent {
    HiveEvent::ToolResult {
        agent: AgentId(agent),
        label: label.into(),
        ok,
        text: text.into(),
        ms,
    }
}

#[test]
fn a_call_opens_a_pending_line_and_its_result_completes_it() {
    let mut t = ToolLines::default();
    t.absorb(&call(7, "fs:read", r#"{"path":"src/foo.rs"}"#), 1_000);
    assert_eq!(t.pending(), 1, "one call in flight");
    let l = &t.blocks[0].lines[0];
    assert_eq!(
        (l.label.as_str(), l.args_short.as_str()),
        ("fs:read", "src/foo.rs")
    );
    assert_eq!(l.started_ms, 1_000);
    t.absorb(&result(7, "fs:read", true, 120, "use std;"), 1_120);
    assert_eq!(t.pending(), 0, "the result closed it");
    let d = t.blocks[0].lines[0].done.as_ref().expect("done");
    assert!(d.ok && d.ms == 120 && d.text == "use std;");
}

#[test]
fn overlapping_calls_complete_oldest_first_and_only_by_label() {
    let mut t = ToolLines::default();
    t.absorb(&call(7, "fs:read", r#"{"path":"a"}"#), 0);
    t.absorb(&call(7, "fs:read", r#"{"path":"b"}"#), 0);
    t.absorb(&call(7, "sys:run", r#"{"cmd":"ls"}"#), 0);
    t.absorb(&result(7, "fs:read", true, 5, ""), 5);
    let done: Vec<bool> = t.blocks[0].lines.iter().map(|l| l.done.is_some()).collect();
    assert_eq!(
        done,
        [true, false, false],
        "the OLDEST fs:read, not sys:run"
    );
    t.absorb(&result(7, "fs:read", false, 9, "gone"), 9);
    let done: Vec<bool> = t.blocks[0].lines.iter().map(|l| l.done.is_some()).collect();
    assert_eq!(done, [true, true, false], "then the next fs:read");
    assert_eq!(t.pending(), 1);
    assert_eq!(t.blocks[0].tally(), (3, 1, 14), "(calls, failed, ms)");
}

#[test]
fn the_subject_is_the_headline_field_the_only_field_or_the_raw_args() {
    let mut t = ToolLines::default();
    t.absorb(
        &call(1, "sys:run", r#"{"cmd":"cargo  test\n--all","cwd":"/x"}"#),
        0,
    );
    t.absorb(&call(1, "x:y", r#"{"query":"crew"}"#), 0);
    t.absorb(&call(1, "x:z", r#"{"needle":"pin"}"#), 0);
    t.absorb(&call(1, "x:w", "{}"), 0);
    t.absorb(&call(1, "x:v", "not json at all"), 0);
    let subj: Vec<&str> = t.blocks[0]
        .lines
        .iter()
        .map(|l| l.args_short.as_str())
        .collect();
    assert_eq!(
        subj,
        ["cargo test --all", "crew", "pin", "", "not json at all"]
    );
    t.absorb(
        &call(1, "x:u", &format!("{{\"a\":\"{}\"}}", "x".repeat(200))),
        0,
    );
    let long = &t.blocks[0].lines[5].args_short;
    assert_eq!(long.chars().count(), 60, "clipped: {long}");
    assert!(long.ends_with('\u{2026}'));
}

#[test]
fn a_block_is_named_agent_n_until_the_broker_names_it() {
    let mut t = ToolLines::default();
    t.absorb(&call(7, "fs:read", "{}"), 0);
    assert_eq!(t.blocks[0].agent, "agent-7");
    // The broker's `Activity { from: "hive" }` names the last-seen agent.
    t.bind("coder");
    assert_eq!(t.blocks[0].agent, "coder");
    // …and later blocks of the same agent are born named.
    t.settle("coder", "10");
    t.absorb(&call(7, "fs:read", "{}"), 0);
    assert_eq!(t.blocks[1].agent, "coder");
    assert!(!t.blocks[1].settled);
}

#[test]
fn settling_anchors_the_open_block_collapsed_and_the_next_call_opens_a_new_one() {
    let mut t = ToolLines::default();
    t.absorb(&call(7, "fs:read", "{}"), 0);
    t.bind("coder");
    t.blocks[0].expanded = true;
    t.settle("coder", "5000");
    let b = &t.blocks[0];
    assert_eq!(b.anchor.as_deref(), Some("5000"));
    assert!(b.settled && !b.expanded, "collapsed above the reply");
    t.settle("coder", "6000");
    assert_eq!(
        t.blocks[0].anchor.as_deref(),
        Some("5000"),
        "already anchored"
    );
    t.absorb(&call(7, "sys:run", "{}"), 0);
    assert_eq!(t.blocks.len(), 2, "a fresh block for the next turn");
}

#[test]
fn abandon_fails_every_pending_line_and_closes_the_open_blocks() {
    let mut t = ToolLines::default();
    t.absorb(&call(7, "fs:read", "{}"), 1_000);
    t.absorb(&call(8, "sys:run", "{}"), 2_000);
    t.absorb(&result(8, "sys:run", true, 1, "ok"), 2_001);
    t.abandon(4_000);
    assert_eq!(t.pending(), 0);
    let d = t.blocks[0].lines[0].done.as_ref().unwrap();
    assert!(!d.ok && d.ms == 3_000, "failed after the 3s it waited");
    let d = t.blocks[1].lines[0].done.as_ref().unwrap();
    assert!(d.ok, "a finished line keeps its outcome");
    assert!(t.blocks.iter().all(|b| b.settled && b.anchor.is_none()));
}
