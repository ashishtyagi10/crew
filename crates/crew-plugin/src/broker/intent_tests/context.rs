//! The run says what it brings — one line after the routing line and before
//! the arm's first event, each part only when there is one, nothing at all
//! when there is none.
use super::*;
use crate::broker::intent::context::{skills_for, ContextLine};
use crate::broker::toolpick::BUDGET;

fn with(f: impl FnOnce(&mut ContextLine)) -> Option<String> {
    let mut c = ContextLine::default();
    f(&mut c);
    c.line()
}

#[test]
fn an_empty_world_and_a_fresh_session_say_nothing() {
    assert_eq!(ContextLine::default().line(), None);
    // Tools under the choosing budget are not news: nothing is chosen.
    assert_eq!(with(|c| c.tools = BUDGET), None);
}

#[test]
fn each_part_appears_only_when_present_with_its_own_plural() {
    assert_eq!(
        with(|c| c.turns = 1).as_deref(),
        Some("context: 1 earlier turn")
    );
    assert_eq!(
        with(|c| c.turns = 2).as_deref(),
        Some("context: 2 earlier turns")
    );
    assert_eq!(with(|c| c.notes = 1).as_deref(), Some("context: a note"));
    assert_eq!(with(|c| c.notes = 3).as_deref(), Some("context: 3 notes"));
    assert_eq!(
        with(|c| c.skills = vec!["a".into(), "b".into()]).as_deref(),
        Some("context: skill a, b")
    );
    assert_eq!(with(|c| c.tools = 41).as_deref(), Some("context: 41 tools"));
    assert_eq!(
        with(|c| c.dirty = 1).as_deref(),
        Some("context: tree dirty (1 file)")
    );
    assert_eq!(
        with(|c| c.dirty = 3).as_deref(),
        Some("context: tree dirty (3 files)")
    );
}

#[test]
fn a_recall_is_said_with_how_far_back_it_reached() {
    let day_ms = 86_400_000;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    assert_eq!(
        with(|c| c.recalled = Some((1, now - 2 * day_ms))).as_deref(),
        Some("context: recalled 1 turn from 2d ago")
    );
    assert_eq!(
        with(|c| c.recalled = Some((3, now - 30 * day_ms))).as_deref(),
        Some("context: recalled 3 turns from 4w ago")
    );
}

#[test]
fn a_session_with_a_graph_announces_the_recall_the_task_will_carry() {
    let session = crate::broker::session::Session::default();
    crate::broker::recall::lock(&session.recall).record(
        "why does the linecap ratchet fail",
        "because a file on the debt list grew",
    );
    let line = ContextLine::gather(
        "the linecap ratchet is red again",
        &session,
        &World::default(),
        &[],
    )
    .expect("a session that remembers says so");
    assert!(line.contains("recalled 1 turn from just now"), "{line}");
}

#[test]
fn a_recall_that_quoted_nothing_is_not_announced() {
    // The graph answered, but the budget left no room for a turn: the line
    // must not say a memory rode in front of the task when none did.
    assert_eq!(with(|c| c.recalled = Some((0, 0))), None);
}

#[test]
fn every_part_together_is_one_line_in_a_fixed_order() {
    let line = with(|c| {
        c.turns = 2;
        c.recalled = None;
        c.notes = 1;
        c.skills = vec!["code-review".into()];
        c.tools = 41;
        c.dirty = 3;
    });
    assert_eq!(
        line.as_deref(),
        Some(
            "context: 2 earlier turns \u{b7} a note \u{b7} skill code-review \u{b7} 41 tools \
             \u{b7} tree dirty (3 files)"
        )
    );
}

#[test]
fn the_context_line_lands_after_the_routing_line_and_before_the_arm() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    crate::broker::thread::lock(&session.thread).record("first ask", "first answer");
    let call = |_: &str| Ok("SHAPE: swarm\nWHY: multi-part work".to_string());
    let mut evs = Vec::new();
    route_with(
        "now the tests too",
        Some(&call),
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    let routing = evs
        .iter()
        .position(|e| text_of(e).starts_with("routing: "))
        .unwrap();
    let context = evs
        .iter()
        .position(|e| text_of(e).starts_with("context: "))
        .expect("a session with a turn says so");
    let arm = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::HivePlan { .. }))
        .expect("the swarm arm ran");
    assert!(routing < context && context < arm, "{evs:?}");
    assert!(
        text_of(&evs[context]).contains("1 earlier turn"),
        "{}",
        text_of(&evs[context])
    );
    let lines = evs
        .iter()
        .filter(|e| text_of(e).starts_with("context: "))
        .count();
    assert_eq!(lines, 1, "said once per turn: {evs:?}");
}

/// The line names the playbooks the ARM will frame — the same pick on the
/// same text, so the decider's memo answers the arm — and nothing for an arm
/// that frames on a different text or none at all.
#[test]
fn skills_are_named_only_for_the_arms_that_frame_the_raw_task() {
    let _g = testenv::mock("ok");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::create_dir_all(dir.join(".crew/skills")).unwrap();
    std::fs::write(
        dir.join(".crew/skills/ship-notes.md"),
        "---\nname: ship-notes\ndescription: write release notes\n---\nList the changes.",
    )
    .unwrap();
    let session = Session::new();
    let task = "write the ship-notes for this release";
    for shape in [Shape::Swarm, Shape::Loop, Shape::Goal, Shape::Reply] {
        assert!(
            skills_for(shape, task, &session).contains(&"ship-notes".to_string()),
            "{shape:?} frames the raw task"
        );
    }
    for shape in [Shape::Fan, Shape::Plan, Shape::Commit, Shape::Review] {
        assert!(
            skills_for(shape, task, &session).is_empty(),
            "{shape:?} frames no skills on the raw task"
        );
    }
    // A reply on a thread is framed on the thread's wrap, not the task.
    crate::broker::thread::lock(&session.thread).record("earlier", "answer");
    assert!(skills_for(Shape::Reply, task, &session).is_empty());
    // The line carries the pick.
    let line = ContextLine::gather(
        "ship it",
        &session,
        &World::default(),
        &["ship-notes".into()],
    );
    assert!(
        line.as_deref()
            .is_some_and(|l| l.contains("skill ship-notes")),
        "{line:?}"
    );
}

#[test]
fn the_project_instruction_files_are_named_on_the_line() {
    assert_eq!(
        with(|c| c.instructions = vec!["AGENTS.md".into()]).as_deref(),
        Some("context: AGENTS.md")
    );
    let both = with(|c| {
        c.instructions = vec!["AGENTS.md".into(), "CLAUDE.md".into()];
        c.notes = 1;
    });
    assert_eq!(
        both.as_deref(),
        Some("context: AGENTS.md, CLAUDE.md \u{b7} a note")
    );
}

#[test]
fn plan_first_is_the_first_thing_the_context_line_says() {
    let line = with(|c| {
        c.plan_first = true;
        c.turns = 2;
    });
    assert_eq!(
        line.as_deref(),
        Some("context: 2 earlier turns \u{b7} plan first"),
        "the mode must be on the line when it is on"
    );
    assert_eq!(with(|c| c.plan_first = false), None);
}
