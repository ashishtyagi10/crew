use super::*;
use crate::broker::testenv;

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

fn any_text(evs: &[PluginEvent], needle: &str) -> bool {
    evs.iter().any(|e| text_of(e).contains(needle))
}

/// Run `dispatch` for `shape` on a fresh session, collecting every event.
fn dispatch_collect(shape: Shape, task: &str) -> (Vec<PluginEvent>, Session) {
    let mut session = Session::new();
    let mut evs = Vec::new();
    dispatch(
        shape,
        task,
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    (evs, session)
}

/// Run the real `route` entry on a fresh session, collecting every event.
fn route_collect(task: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    route(
        task,
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

// ── grammar ────────────────────────────────────────────────────────────────

#[test]
fn parses_every_shape_token() {
    for (line, shape) in [
        ("SHAPE: reply", Shape::Reply),
        ("SHAPE: fan", Shape::Fan),
        ("SHAPE: loop", Shape::Loop),
        ("SHAPE: plan", Shape::Plan),
        ("SHAPE: swarm", Shape::Swarm),
    ] {
        assert_eq!(parse_shape(line), Some(shape), "{line}");
    }
}

#[test]
fn grammar_is_case_insensitive_and_tolerates_padding() {
    assert_eq!(parse_shape("shape: FAN"), Some(Shape::Fan));
    assert_eq!(parse_shape("  SHAPE:  plan  "), Some(Shape::Plan));
    assert_eq!(parse_shape("\n\nSHAPE: loop"), Some(Shape::Loop));
}

#[test]
fn prose_after_the_first_line_is_tolerated() {
    assert_eq!(
        parse_shape("SHAPE: fan\nbecause the user wants everyone's take"),
        Some(Shape::Fan)
    );
}

#[test]
fn trailing_punctuation_on_the_token_is_tolerated() {
    assert_eq!(parse_shape("SHAPE: reply."), Some(Shape::Reply));
}

#[test]
fn garbage_parses_to_none_never_a_guess() {
    for bad in [
        "",
        "fan",
        "I would fan out here",
        "SHAPE:",
        "SHAPE: banana",
        "SHAPES: fan",
        "the shape is: fan",
    ] {
        assert_eq!(parse_shape(bad), None, "{bad:?}");
    }
}

// ── the classifier seam ────────────────────────────────────────────────────

#[test]
fn classify_sends_the_task_and_the_grammar_to_the_model() {
    let seen = std::sync::Mutex::new(String::new());
    let call = |p: &str| {
        *seen.lock().unwrap() = p.to_string();
        Ok("SHAPE: plan".to_string())
    };
    assert_eq!(
        classify_with("refactor the config parser", &call),
        Some(Shape::Plan)
    );
    let p = seen.lock().unwrap();
    assert!(p.contains("refactor the config parser"), "{p}");
    assert!(p.contains("SHAPE: <reply|fan|loop|plan|swarm>"), "{p}");
}

#[test]
fn classify_call_error_is_none() {
    let call = |_: &str| Err("boom".to_string());
    assert_eq!(classify_with("x", &call), None);
}

#[test]
fn classify_off_grammar_reply_is_none() {
    let call = |_: &str| Ok("I'd fan out for this one".to_string());
    assert_eq!(classify_with("x", &call), None);
}

// ── dispatch: each shape reaches its own capability path ───────────────────

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

// ── fallbacks: every stopped classifier is today's swarm ───────────────────

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
