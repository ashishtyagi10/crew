//! Registering, listing and cancelling a standing intent over the wire — the three requests the
//! `crew daemon at | watching | cancel` CLI is a thin face over.
use super::tests::rig;
use crate::daemon::answer;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

/// Far enough in the future that the test is not a clock race.
const LATER: u64 = 9_000_000_000_000;

#[test]
fn registering_listing_and_cancelling_go_through_the_daemon() {
    let mut r = rig("ipc");
    let id = match answer(
        &Request::Watch {
            v: PROTOCOL_V,
            text: "the forecast".into(),
            to: "test:me".into(),
            fire_ms: LATER,
            repeat_secs: Some(86_400),
        },
        &mut r.d,
    ) {
        Some(Reply::Watched { id, fire_ms }) => {
            assert_eq!(fire_ms, LATER);
            id
        }
        other => panic!("expected a watched reply, got {other:?}"),
    };
    match answer(&Request::Watching { v: PROTOCOL_V }, &mut r.d) {
        Some(Reply::Watchlist { intents }) => {
            assert_eq!(intents.len(), 1);
            assert_eq!(intents[0].id, id);
            assert_eq!(intents[0].text, "the forecast");
            assert_eq!(intents[0].to, "test:me");
            assert_eq!(intents[0].repeat, "daily", "the cadence reads as a word");
        }
        other => panic!("expected a watchlist, got {other:?}"),
    }
    match answer(
        &Request::Unwatch {
            v: PROTOCOL_V,
            id: id.clone(),
        },
        &mut r.d,
    ) {
        Some(Reply::Unwatched { found: true, .. }) => {}
        other => panic!("expected a cancellation, got {other:?}"),
    }
    match answer(&Request::Watching { v: PROTOCOL_V }, &mut r.d) {
        Some(Reply::Watchlist { intents }) => assert!(intents.is_empty()),
        other => panic!("expected a watchlist, got {other:?}"),
    }
}

#[test]
fn cancelling_something_that_is_not_watched_says_so() {
    let mut r = rig("nocancel");
    match answer(
        &Request::Unwatch {
            v: PROTOCOL_V,
            id: "w9".into(),
        },
        &mut r.d,
    ) {
        Some(Reply::Unwatched { found: false, .. }) => {}
        other => panic!("expected found:false, got {other:?}"),
    }
}

#[test]
fn an_intent_with_nowhere_to_answer_is_refused_rather_than_stored() {
    // The loopback channel has no sole address, so there is no default to fall back on. Storing
    // this would be an alarm that fires into nothing at 7am.
    let mut r = rig("nowhere");
    match answer(
        &Request::Watch {
            v: PROTOCOL_V,
            text: "the forecast".into(),
            to: String::new(),
            fire_ms: LATER,
            repeat_secs: None,
        },
        &mut r.d,
    ) {
        Some(Reply::Failed { message }) => assert!(message.contains("--to"), "{message}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(r.watch.live().is_empty(), "and nothing was written");
}
