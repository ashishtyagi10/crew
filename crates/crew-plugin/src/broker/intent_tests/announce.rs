//! The pane is told the routing decision — before the dispatched arm's first
//! event, with the model's reason when it gave one, and with the honest name
//! of every stop that fell back to the swarm.
use super::*;

/// Position of the first `Message` whose text contains `needle`.
fn pos_text(evs: &[PluginEvent], needle: &str) -> Option<usize> {
    evs.iter().position(|e| text_of(e).contains(needle))
}

/// Position of the first agent-smith `Activity` in `state`.
fn pos_smith(evs: &[PluginEvent], want: &str) -> Option<usize> {
    evs.iter().position(|e| {
        matches!(e, PluginEvent::Activity { agent, state, .. }
            if agent == "agent smith" && state == want)
    })
}

/// The routing line, exactly, and only one of them.
fn routing_line(evs: &[PluginEvent]) -> String {
    let lines: Vec<&str> = evs
        .iter()
        .map(text_of)
        .filter(|t| t.starts_with("routing: "))
        .collect();
    assert_eq!(lines.len(), 1, "exactly one routing line: {evs:?}");
    lines[0].to_string()
}

#[test]
fn the_routing_line_lands_before_the_dispatched_arm_speaks() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: fan\nWHY: the user wants many takes".to_string());
    let evs = route_stubbed("have every agent take a crack at the login bug", &call);
    assert_eq!(
        routing_line(&evs),
        "routing: fan — the user wants many takes"
    );
    let said = pos_text(&evs, "routing: fan").unwrap();
    let arm = pos_text(&evs, "fanning out to 3 agents").expect("the fan arm ran");
    assert!(
        said < arm,
        "routing line at {said}, arm's first line at {arm}: {evs:?}"
    );
}

#[test]
fn the_routing_activity_brackets_the_routing_line() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: reply".to_string());
    let evs = route_stubbed("hello there", &call);
    let thinking = pos_smith(&evs, "thinking").expect("smith goes thinking while routing");
    let said = pos_text(&evs, "routing: reply").unwrap();
    let idle = pos_smith(&evs, "idle").expect("smith settles after deciding");
    let arm = pos_text(&evs, "starting with planner").expect("the relay arm ran");
    assert!(thinking < said && said < idle && idle < arm, "{evs:?}");
    assert_eq!(
        thinking, 0,
        "nothing precedes the routing activity: {evs:?}"
    );
}

#[test]
fn a_shape_with_no_reason_is_said_bare() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: reply".to_string());
    let evs = route_stubbed("hello there", &call);
    assert_eq!(routing_line(&evs), "routing: reply");
}

#[test]
fn a_sized_loop_says_its_count_on_the_routing_line() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: loop\nWHY: polish until it reads well\nROUNDS: 5".to_string());
    let evs = route_stubbed("keep polishing the intro", &call);
    assert_eq!(
        routing_line(&evs),
        "routing: loop \u{d7}5 — polish until it reads well"
    );
}

#[test]
fn a_subset_fan_names_its_agents_on_the_routing_line() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call =
        |_: &str| Ok("SHAPE: fan\nWHY: two views wanted\nAGENTS: coder, reviewer".to_string());
    let evs = route_stubbed("what do the builders think?", &call);
    assert_eq!(
        routing_line(&evs),
        "routing: fan \u{2192} coder, reviewer — two views wanted"
    );
}

#[test]
fn no_classifier_says_classifier_off_and_swarms() {
    let _g = testenv::mock("ok");
    // `route` under the mock provider resolves to no classifier at all.
    let evs = route_collect("do something useful");
    assert_eq!(routing_line(&evs), "routing: swarm — classifier off");
    let said = pos_text(&evs, "routing: swarm").unwrap();
    let plan = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::HivePlan { .. }))
        .expect("the swarm arm ran");
    assert!(said < plan, "{evs:?}");
}

#[test]
fn a_failed_call_says_so_and_swarms() {
    let _g = testenv::mock("ok");
    let call = |_: &str| Err("provider timed out".to_string());
    let evs = route_stubbed("do something useful", &call);
    assert_eq!(
        routing_line(&evs),
        "routing: swarm — classifier failed: provider timed out"
    );
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "{evs:?}"
    );
}

#[test]
fn an_off_grammar_reply_says_so_and_swarms() {
    let _g = testenv::mock("ok");
    let call = |_: &str| Ok("I'd fan out for this one".to_string());
    let evs = route_stubbed("do something useful", &call);
    assert_eq!(
        routing_line(&evs),
        "routing: swarm — classifier reply was off-grammar"
    );
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
        "{evs:?}"
    );
}

/// [`route_with`] on `session` (not a fresh one — the gate tests need the
/// pending plan to survive between turns), collecting every event.
fn route_on(session: &mut Session, task: &str, call: Classifier) -> Vec<PluginEvent> {
    let mut evs = Vec::new();
    route_with(
        task,
        Some(call),
        session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

#[test]
fn a_human_gate_answer_is_never_routed_so_it_gets_no_routing_line() {
    let _g = testenv::mock_with_specialists("1. step one\n2. step two", testenv::TRIO);
    let mut session = Session::new();
    let plan = |_: &str| Ok("SHAPE: plan".to_string());
    let evs = route_on(&mut session, "migrate the config", &plan);
    assert_eq!(routing_line(&evs), "routing: plan");
    // The plan is pending; "no" is the user's own verdict, not a message to
    // classify — the classifier must not even be asked.
    let never = |_: &str| -> Result<String, String> { panic!("the gate answered, not the model") };
    let evs = route_on(&mut session, "no", &never);
    assert!(!evs.is_empty(), "the gate must answer the verdict: {evs:?}");
    assert!(
        !evs.iter().any(|e| text_of(e).starts_with("routing: ")),
        "{evs:?}"
    );
}
