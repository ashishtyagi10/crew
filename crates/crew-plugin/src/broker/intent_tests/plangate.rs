//! The conversational plan gate: with a plan pending, the user's own verdict
//! word runs or discards it — matched deterministically, BEFORE any classify
//! call, mirroring the commit "apply" gate — and anything else is a normal
//! message that leaves the plan pending. No pending plan → the gate words are
//! ordinary messages.
use super::*;
use crate::broker::plan::PendingPlan;

/// [`route_stubbed`], but on a caller-owned session — the gate spans two
/// sends (draft, then verdict), which must share the pending plan.
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

fn pend(session: &Session) {
    *session.plan.lock().unwrap() = Some(PendingPlan {
        task: "migrate the config".into(),
        plan: "1. read the old file\n2. write the new one".into(),
        verify: false,
    });
}

fn plan_pending(session: &Session) -> bool {
    session.plan.lock().unwrap().is_some()
}

#[test]
fn approve_words_run_the_pending_plan_as_a_swarm_before_any_classification() {
    for word in ["approve", "go", "run it", "go ahead", "do it", "yes"] {
        let _g = testenv::mock("done");
        let mut session = Session::new();
        pend(&session);
        // The classifier is a saboteur that would misroute the verdict — the
        // gate must win BEFORE any model call.
        let saboteur = |_: &str| Ok("SHAPE: swarm".to_string());
        let evs = route_on(&mut session, word, &saboteur);
        assert!(
            any_text(&evs, "running the approved plan as a swarm"),
            "{word}: {evs:?}"
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, PluginEvent::HivePlan { .. })),
            "{word}: the swarm ran: {evs:?}"
        );
        assert!(
            !any_text(&evs, "routing:"),
            "{word}: the gate answered, no routing line was drawn: {evs:?}"
        );
        assert!(!plan_pending(&session), "{word}: the plan was consumed");
    }
}

/// `VERIFY: yes` on a `plan` routing is kept with the draft — the plan is
/// judged when it RUNS, which is a later send, so the flag has to wait on
/// the session with the plan it was routed for.
#[test]
fn a_plan_routed_with_verify_yes_is_stored_with_verify() {
    let _g = testenv::mock_with_specialists("1. fix\n2. test\n@done", testenv::TRIO);
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: plan\nVERIFY: yes".to_string());
    let evs = route_on(&mut session, "make the tests pass", &call);
    assert!(any_text(&evs, "routing: plan \u{00b7} verified"), "{evs:?}");
    let held = session.plan.lock().unwrap();
    let p = held.as_ref().expect("the plan is pending");
    assert!(p.verify, "the draft carries the check for its run");
    assert_eq!(p.task, "make the tests pass");

    drop(held);
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: plan".to_string());
    route_on(&mut session, "migrate the config", &call);
    let held = session.plan.lock().unwrap();
    assert!(!held.as_ref().unwrap().verify, "no line, no check");
}

#[test]
fn reject_words_discard_the_pending_plan_before_any_classification() {
    for word in ["reject", "no", "drop it", "discard"] {
        let _g = testenv::mock_with_specialists("done\n@done", testenv::TRIO);
        let mut session = Session::new();
        pend(&session);
        let saboteur = |_: &str| Ok("SHAPE: swarm".to_string());
        let evs = route_on(&mut session, word, &saboteur);
        assert!(any_text(&evs, "plan discarded"), "{word}: {evs:?}");
        assert!(!plan_pending(&session), "{word}: the plan was dropped");
    }
}

#[test]
fn a_non_gate_message_routes_normally_and_the_plan_stays_pending() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    pend(&session);
    let call = |_: &str| Ok("SHAPE: reply".to_string());
    let evs = route_on(&mut session, "also add tests please", &call);
    assert!(any_text(&evs, "starting with planner"), "{evs:?}");
    assert!(!any_text(&evs, "running the approved plan"), "{evs:?}");
    assert!(!any_text(&evs, "plan discarded"), "{evs:?}");
    assert!(
        plan_pending(&session),
        "the plan must survive an unrelated send"
    );
}

#[test]
fn gate_words_with_nothing_pending_are_ordinary_messages() {
    for word in ["approve", "reject", "drop it", "run it"] {
        let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
        let mut session = Session::new();
        let call = |_: &str| Ok("SHAPE: reply".to_string());
        let evs = route_on(&mut session, word, &call);
        assert!(any_text(&evs, "starting with planner"), "{word}: {evs:?}");
        assert!(
            !any_text(&evs, "running the approved plan"),
            "{word}: {evs:?}"
        );
        assert!(!any_text(&evs, "plan discarded"), "{word}: {evs:?}");
    }
}
