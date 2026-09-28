//! Where a stumbling router lands, and how long it may stumble. A call that
//! failed or a reply off-grammar twice used to be the swarm — planner,
//! workers, closing answer — for what was usually a one-line question, after
//! up to 30 s of waiting on the router. Now it is one agent's reply, after at
//! most 12 s; a router that may not run at all is still the swarm.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::*;
use crate::broker::intent::classify::{CLASSIFY_TIMEOUT, ONESHOT_TIMEOUT};

#[test]
fn a_failed_router_call_is_one_agent_s_reply_and_says_why() {
    let down = |_: &str| -> Result<String, String> { Err("timed out".into()) };
    let r = decide_in("what does clip() do?", &World::default(), Some(&down));
    assert_eq!(r.decision().shape, Shape::Reply, "{r:?}");
    let line = r.line();
    assert!(
        line.starts_with("routing: reply — classifier failed"),
        "{line}"
    );
    assert!(line.ends_with("timed out"), "the error is carried: {line}");
}

#[test]
fn prose_twice_is_one_agent_s_reply_after_exactly_two_asks() {
    let n = AtomicUsize::new(0);
    let prose = |_: &str| -> Result<String, String> {
        n.fetch_add(1, Ordering::SeqCst);
        Ok("Happy to help! This looks like a quick question.".into())
    };
    let r = decide_in("what does clip() do?", &World::default(), Some(&prose));
    assert_eq!(r.decision().shape, Shape::Reply, "{r:?}");
    assert_eq!(
        r.line(),
        "routing: reply — classifier reply was off-grammar"
    );
    assert_eq!(n.load(Ordering::SeqCst), 2, "asked twice, no more");
}

#[test]
fn a_router_that_may_not_run_is_still_the_swarm() {
    assert_eq!(Routing::Off.decision().shape, Shape::Swarm);
    assert_eq!(Routing::Off.line(), "routing: swarm — classifier off");
}

/// The fallback reply is the same reply a chosen `SHAPE: reply` with no
/// `AGENTS:` line gets — same agent dialled first — so a stumble changes
/// nothing about who answers.
#[test]
fn the_fallback_reply_dials_whoever_a_chosen_reply_would() {
    let first_dial = |call: Classifier| {
        let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
        let evs = route_stubbed("what does clip() do?", call);
        evs.iter()
            .find_map(|e| match e {
                PluginEvent::Activity { agent, state, .. }
                    if state == "thinking" && agent != "agent smith" =>
                {
                    Some(agent.clone())
                }
                _ => None,
            })
            .expect("an agent was dialled")
    };
    let chosen = first_dial(&|_: &str| Ok("SHAPE: reply".to_string()));
    let failed = first_dial(&|_: &str| Err("boom".to_string()));
    let junk = first_dial(&|_: &str| Ok("no idea".to_string()));
    assert_eq!(failed, chosen);
    assert_eq!(junk, chosen);
}

/// The router's bound is its own: 12 s, over ten times its measured ~1 s.
/// Everything else on the plumbing — election, the summarizer, the judge,
/// the swarm's closing answer — keeps the 30 s it had, because the
/// summarizer and the closing answer write paragraphs.
#[test]
fn only_the_router_s_bound_is_shortened() {
    assert_eq!(CLASSIFY_TIMEOUT, Duration::from_secs(12));
    assert_eq!(ONESHOT_TIMEOUT, Duration::from_secs(30));
}
