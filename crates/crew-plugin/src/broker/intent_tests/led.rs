//! A turn with a lead (a signed-in CLI) still passes the gates, and a review
//! ask reaches the review whoever leads, before any model is asked.
use super::capability::{git, seed_repo};
use super::gate::led;
use super::*;

fn led_collect(session: &mut Session, task: &str) -> Vec<PluginEvent> {
    let mut evs = Vec::new();
    let tick = crate::broker::tick::noop_tick_emit();
    led(task, "planner", session, &tick, &mut |ev| {
        evs.push(ev);
        Ok(())
    })
    .unwrap();
    evs
}

/// Loop 39 S8, live: "review my diff" went to the lead as an open task, and
/// a peer edited the callers it should have reported.
#[test]
fn a_led_review_ask_reaches_the_review_not_the_lead() {
    let _g = testenv::mock_with_specialists("no findings", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let evs = led_collect(&mut session, "review my diff");
    assert!(any_text(&evs, "reviewing the working diff"), "{evs:?}");
    assert!(any_text(&evs, "no findings"), "{evs:?}");
}

#[test]
fn a_led_undo_ask_is_offered_not_dialled() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let evs = led_collect(&mut session, "undo that");
    assert!(pos_dial(&evs, "planner").is_none(), "{evs:?}");
}

#[test]
fn a_led_task_still_goes_to_the_lead() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let dir = seed_repo();
    let mut session = Session::new();
    let evs = led_collect(&mut session, "review my diff and fix what you find");
    assert!(pos_dial(&evs, "planner").is_some(), "{evs:?}");
    assert!(!any_text(&evs, "reviewing the"), "{evs:?}");
    assert!(!git(&dir, &["status", "--porcelain"]).is_empty());
}

/// The routed path reads the ask before the classifier: a router that would
/// call it a swarm never gets the chance.
#[test]
fn a_routed_review_ask_beats_the_classifier() {
    let _g = testenv::mock_with_specialists("no findings", testenv::TRIO);
    seed_repo();
    let saboteur = |_: &str| Ok("SHAPE: swarm".to_string());
    let evs = route_stubbed("look over my changes", &saboteur);
    assert!(any_text(&evs, "reviewing the working diff"), "{evs:?}");
}
