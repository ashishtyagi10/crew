use super::*;
use crate::broker::testenv;

fn texts(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|ev| match ev {
            PluginEvent::Message { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn run(session: &mut Session, cmd: &str, rest: &str) -> Vec<PluginEvent> {
    let mut out = Vec::new();
    let mut emit = |ev| {
        out.push(ev);
        Ok(())
    };
    match cmd {
        "plan" => plan_cmd(session, rest, &mut emit).unwrap(),
        "approve" => {
            approve_cmd(session, &crate::broker::tick::noop_tick_emit(), &mut emit).unwrap()
        }
        "reject" => reject_cmd(session, &mut emit).unwrap(),
        _ => unreachable!(),
    }
    out
}

#[test]
fn plan_without_task_shows_usage() {
    let mut s = Session::new();
    let t = texts(&run(&mut s, "plan", "  "));
    assert!(t[0].contains("usage: /plan"), "{t:?}");
}

#[test]
fn plan_drafts_and_holds_without_executing() {
    let _g = testenv::mock_with_specialists("1. survey\n2. build\n@done", testenv::TRIO);
    let mut s = Session::new();
    let t = texts(&run(&mut s, "plan", "ship the feature"));
    assert!(t[0].contains("nothing runs until /approve"), "{t:?}");
    assert!(t.iter().any(|x| x.contains("1. survey")), "{t:?}");
    assert!(t.last().unwrap().contains("/approve"), "{t:?}");
    let held = s.plan.lock().unwrap();
    let p = held.as_ref().expect("plan stored");
    assert_eq!(p.task, "ship the feature");
    assert!(
        !p.plan.contains("@done"),
        "control line stripped: {}",
        p.plan
    );
}

#[test]
fn approve_without_a_plan_hints_at_plan() {
    let _g = testenv::mock("ok\n@done");
    let mut s = Session::new();
    let t = texts(&run(&mut s, "approve", ""));
    assert!(t[0].contains("no plan pending"), "{t:?}");
}

#[test]
fn approve_runs_the_relay_and_clears_the_plan() {
    let _g = testenv::mock_with_specialists("done as planned\n@done", testenv::TRIO);
    let mut s = Session::new();
    *s.plan.lock().unwrap() = Some(PendingPlan {
        task: "ship it".into(),
        plan: "1. do".into(),
        author: "planner".into(),
    });
    let t = texts(&run(&mut s, "approve", ""));
    assert!(t[0].contains("plan approved"), "{t:?}");
    assert!(
        t.iter().any(|x| x.contains("done as planned")),
        "execution ran: {t:?}"
    );
    assert!(s.plan.lock().unwrap().is_none(), "plan consumed");
}

#[test]
fn reject_discards_a_pending_plan() {
    let mut s = Session::new();
    *s.plan.lock().unwrap() = Some(PendingPlan {
        task: "t".into(),
        plan: "p".into(),
        author: "planner".into(),
    });
    let t = texts(&run(&mut s, "reject", ""));
    assert!(t[0].contains("plan discarded"), "{t:?}");
    assert!(s.plan.lock().unwrap().is_none());
    let t = texts(&run(&mut s, "reject", ""));
    assert!(t[0].contains("no plan pending"), "{t:?}");
}

#[test]
fn prompts_frame_drafting_and_execution() {
    let p = plan_prompt("add dark mode");
    assert!(p.contains("add dark mode"));
    assert!(p.contains("Do NOT execute"));
    let e = execute_body("add dark mode", "1. css\n2. toggle");
    assert!(e.contains("Approved plan:\n1. css"));
}

#[test]
fn strip_control_removes_routing_directives() {
    assert_eq!(strip_control("the plan\n@done"), "the plan");
    assert_eq!(strip_control("plain reply"), "plain reply");
}
