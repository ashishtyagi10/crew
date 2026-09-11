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
        "plan" => plan_cmd(session, rest, false, &mut emit).unwrap(),
        "approve" => approve_cmd(session, &mut emit).unwrap(),
        "reject" => reject_cmd(session, &mut emit).unwrap(),
        _ => unreachable!(),
    }
    out
}

fn pending(task: &str, plan: &str, verify: bool) -> Option<PendingPlan> {
    Some(PendingPlan {
        task: task.into(),
        plan: plan.into(),
        verify,
    })
}

/// The goal the swarm's planner was handed, read off the `HivePlan` event
/// (the mock arm plans with the stub, whose task prompts ARE the goal).
fn planned_goal(evs: &[PluginEvent]) -> Option<String> {
    evs.iter().find_map(|e| match e {
        PluginEvent::HivePlan { tasks } => Some(tasks[0].prompt.clone()),
        _ => None,
    })
}

#[test]
fn plan_without_task_shows_usage() {
    let mut s = Session::new();
    let t = texts(&run(&mut s, "plan", "  "));
    assert!(t[0].contains("nothing to plan"), "{t:?}");
}

#[test]
fn plan_drafts_and_holds_without_executing() {
    let _g = testenv::mock_with_specialists("1. survey\n2. build\n@done", testenv::TRIO);
    let mut s = Session::new();
    let t = texts(&run(&mut s, "plan", "ship the feature"));
    assert!(t[0].contains("nothing runs until you approve"), "{t:?}");
    assert!(t.iter().any(|x| x.contains("1. survey")), "{t:?}");
    // The decision is a keypress now, not a construct to learn.
    assert!(t.last().unwrap().contains("enter runs it"), "{t:?}");

    let held = s.plan.lock().unwrap();
    let p = held.as_ref().expect("plan stored");
    assert_eq!(p.task, "ship the feature");
    assert!(!p.verify, "no VERIFY: yes was routed");
    assert!(
        !p.plan.contains("@done"),
        "control line stripped: {}",
        p.plan
    );
}

/// The router's `VERIFY: yes` rides on the draft, so the run that follows
/// the approval can be judged — the plan struct is where it waits.
#[test]
fn a_draft_routed_with_verify_holds_verify_for_the_run() {
    let _g = testenv::mock_with_specialists("1. survey\n@done", testenv::TRIO);
    let mut s = Session::new();
    let mut sink = |_| Ok(());
    plan_cmd(&mut s, "make the tests pass", true, &mut sink).unwrap();
    let held = s.plan.lock().unwrap();
    assert!(held.as_ref().expect("plan stored").verify);
}

#[test]
fn approve_without_a_plan_hints_at_plan() {
    let _g = testenv::mock("ok\n@done");
    let mut s = Session::new();
    let t = texts(&run(&mut s, "approve", ""));
    assert!(t[0].contains("no plan pending"), "{t:?}");
}

/// Approval is a SWARM run on the approved plan: one line says so, the plan
/// event follows it, and the goal the planner sees leads with the approved
/// steps under the header `PLANNER_SYSTEM` names, then the request.
#[test]
fn approve_runs_the_swarm_on_the_approved_plan_and_clears_it() {
    let _g = testenv::mock("done as planned");
    let mut s = Session::new();
    *s.plan.lock().unwrap() = pending("ship it", "1. survey\n2. build", false);
    let evs = run(&mut s, "approve", "");
    let t = texts(&evs);
    assert_eq!(t[0], "running the approved plan as a swarm", "{t:?}");
    assert!(
        !t.iter().any(|x| x.contains("leads execution")),
        "the relay wording is gone: {t:?}"
    );
    let said = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::Message { .. }))
        .unwrap();
    let planned = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::HivePlan { .. }))
        .expect("the approval ran the swarm: a HivePlan was emitted");
    assert!(said < planned, "the approval line lands before the plan");
    // This machine's own skills may wrap the goal in their roster (the
    // frame is compared against, not spelled out); the approved goal is the
    // TASK the frame ends with, byte for byte.
    let goal = planned_goal(&evs).unwrap();
    let at = goal
        .find("APPROVED PLAN")
        .expect("the header is in the goal");
    assert_eq!(&goal[at..], approved_goal("ship it", "1. survey\n2. build"));
    assert!(goal.ends_with("GOAL:\nship it"), "{goal}");
    assert!(
        t.iter().any(|x| x.contains("done as planned")),
        "the crew ran: {t:?}"
    );
    assert!(s.plan.lock().unwrap().is_none(), "plan consumed");
}

/// A draft's `verify` is what the approval hands the run — the struct is the
/// only carrier between the two sends, so the field must survive the take.
#[test]
fn the_approved_plan_hands_its_verify_to_the_run() {
    let _g = testenv::mock("done");
    let mut s = Session::new();
    *s.plan.lock().unwrap() = pending("make the tests pass", "1. fix", true);
    let taken = s.plan.lock().unwrap().take().unwrap();
    assert!(taken.verify);
    *s.plan.lock().unwrap() = Some(taken);
    // Under the mock there is no judge to observe (routing's gates), so the
    // run itself is asserted through the framed goal; the flag's journey to
    // `run_task` is one field read in `approve_cmd`.
    let goal = planned_goal(&run(&mut s, "approve", "")).unwrap();
    assert!(goal.contains("GOAL:\nmake the tests pass"), "{goal}");
}

#[test]
fn reject_discards_a_pending_plan() {
    let mut s = Session::new();
    *s.plan.lock().unwrap() = pending("t", "p", false);
    let t = texts(&run(&mut s, "reject", ""));
    assert!(t[0].contains("plan discarded"), "{t:?}");
    assert!(s.plan.lock().unwrap().is_none());
    let t = texts(&run(&mut s, "reject", ""));
    assert!(t[0].contains("no plan pending"), "{t:?}");
}

#[test]
fn prompts_frame_drafting_and_the_approved_goal() {
    let p = plan_prompt("add dark mode");
    assert!(p.contains("add dark mode"));
    assert!(p.contains("Do NOT execute"));
    let g = approved_goal("add dark mode", "1. css\n2. toggle");
    assert!(
        g.starts_with("APPROVED PLAN \u{2014} follow these steps"),
        "{g}"
    );
    assert!(
        g.contains("dependencies:\n1. css\n2. toggle\n\nGOAL:\nadd dark mode"),
        "{g}"
    );
    let plan = g.find("1. css").unwrap();
    let goal = g.find("GOAL:").unwrap();
    assert!(plan < goal, "the plan heads the goal: {g}");
}

#[test]
fn strip_control_removes_routing_directives() {
    assert_eq!(strip_control("the plan\n@done"), "the plan");
    assert_eq!(strip_control("plain reply"), "plain reply");
}

/// The host must learn a plan is pending from an EVENT, not by matching prose
/// — that is what lets the pane bind enter/esc to it instead of teaching two
/// constructs. Approving and rejecting must both clear it, or the pane would
/// keep offering a decision that has already been made.
#[test]
fn the_pending_plan_is_announced_and_cleared() {
    let _g = testenv::mock_with_specialists("1. survey\n2. build\n@done", testenv::TRIO);
    let pending = |evs: &[PluginEvent]| -> Vec<bool> {
        evs.iter()
            .filter_map(|e| match e {
                PluginEvent::Plan { pending } => Some(*pending),
                _ => None,
            })
            .collect()
    };
    let mut s = Session::new();
    assert_eq!(pending(&run(&mut s, "plan", "ship it")), vec![true]);
    assert_eq!(pending(&run(&mut s, "reject", "")), vec![false]);

    let mut s = Session::new();
    run(&mut s, "plan", "ship it");
    assert_eq!(pending(&run(&mut s, "approve", "")), vec![false]);
}
