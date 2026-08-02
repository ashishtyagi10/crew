//! Each shape reaches its own capability path; every stopped classifier is
//! today's swarm; and retired commands keep plain-language parity.
use super::*;
use crate::broker::testenv;

#[test]
fn reply_shape_reaches_the_relay_not_the_swarm() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let (evs, _) = dispatch_collect(Shape::Reply, "hello there");
    assert!(any_text(&evs, "starting with planner"), "{evs:?}");
    assert!(
        !evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "the relay path must not plan a task graph: {evs:?}"
    );
}

#[test]
fn fan_shape_fans_to_every_agent() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let (evs, _) = dispatch_collect(Shape::Fan, "compare approaches");
    assert!(any_text(&evs, "fanning out to 3 agents"), "{evs:?}");
}

#[test]
fn loop_shape_runs_the_default_rounds_on_the_task() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let (evs, _) = dispatch_collect(Shape::Loop, "polish the intro");
    // Both ends of the loop: round 1 announced, and all LOOP_ROUNDS ran —
    // which also pins that the count went in as a count, not as task text.
    assert!(any_text(&evs, "loop round 1/3"), "{evs:?}");
    assert!(any_text(&evs, "3 round(s) complete"), "{evs:?}");
}

#[test]
fn plan_shape_drafts_and_gates_on_approval() {
    let _g = testenv::mock_with_specialists("1. step one\n2. step two", testenv::TRIO);
    let (evs, session) = dispatch_collect(Shape::Plan, "migrate the config");
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::Plan { pending: true })),
        "{evs:?}"
    );
    let held = session.plan.lock().unwrap();
    let p = held
        .as_ref()
        .expect("the drafted plan is held for approval");
    assert_eq!(p.task, "migrate the config");
}

#[test]
fn swarm_shape_plans_a_task_graph() {
    let _g = testenv::mock("ok");
    let (evs, _) = dispatch_collect(Shape::Swarm, "build the feature");
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "{evs:?}"
    );
}

#[test]
fn route_under_the_mock_provider_is_the_swarm_unchanged() {
    let _g = testenv::mock("ok");
    let evs = route_collect("do something useful");
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "{evs:?}"
    );
    assert!(!any_text(&evs, "fanning out"), "{evs:?}");
}

#[test]
fn keyless_route_is_the_swarm_unchanged() {
    let _g = testenv::no_provider();
    let evs = route_collect("do something useful");
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "{evs:?}"
    );
}

#[test]
fn crew_intent_0_disables_and_anything_else_does_not() {
    // testenv::mock holds the process-wide env lock; CREW_INTENT is set and
    // removed strictly inside the guard's lifetime.
    let _g = testenv::mock("ok");
    std::env::set_var("CREW_INTENT", "0");
    let off = disabled();
    std::env::set_var("CREW_INTENT", "1");
    let on = disabled();
    std::env::remove_var("CREW_INTENT");
    assert!(off, "CREW_INTENT=0 must disable classification");
    assert!(!on, "CREW_INTENT=1 must not disable classification");
    assert!(!disabled(), "unset must mean enabled");
}

// ── plain-language parity for the retired /fan and /loop ───────────────────

#[test]
fn a_natural_fan_phrasing_reaches_the_fan_capability_with_no_slash() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: fan".to_string());
    let evs = route_stubbed("have every agent take a crack at the login bug", &call);
    assert!(any_text(&evs, "fanning out to 3 agents"), "{evs:?}");
}

#[test]
fn a_natural_refine_phrasing_reaches_the_loop_capability_with_no_slash() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: loop".to_string());
    let evs = route_stubbed("keep refining the intro until it sings", &call);
    assert!(any_text(&evs, "loop round 1/3"), "{evs:?}");
    assert!(any_text(&evs, "3 round(s) complete"), "{evs:?}");
}

#[test]
fn a_natural_goal_phrasing_reaches_the_judge_loop_with_no_slash() {
    // The mock reply serves both the worker round ("MET: …" becomes the
    // answer) and the judge's verdict, which rules the goal met in round 1.
    let _g = testenv::mock_with_specialists("MET: shipshape\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: goal".to_string());
    let evs = route_stubbed("keep working until the parser round-trips", &call);
    assert!(any_text(&evs, "goal round 1/5"), "{evs:?}");
    assert!(any_text(&evs, "goal met"), "{evs:?}");
}
