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

/// The same three commands, said on a channel instead of typed on the machine — which is where
/// somebody actually is when they think of the errand.
mod from_a_channel {
    use super::super::tests::{rig, sent};
    use crate::channel::Inbound;

    fn say(r: &mut super::super::tests::Rig, text: &str) {
        r.wire.lock().unwrap().inbox.push(Inbound {
            from: "test:me".into(),
            text: text.into(),
        });
        r.d.service_channels();
    }

    #[test]
    fn an_alarm_set_from_a_channel_is_confirmed_and_fires_later() {
        let mut r = rig("chat-set");
        say(&mut r, "remind me tomorrow 9am to call the bank");
        let out = sent(&r);
        assert_eq!(out.len(), 1, "one confirmation");
        assert!(out[0].1.contains("call the bank"), "{}", out[0].1);
        assert!(out[0].1.contains("w1"), "and it names the id: {}", out[0].1);

        let live = r.watch.live();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].to, "test:me", "it answers where it was set");
        let fire = live[0].fire_ms;
        r.d.service_intents(fire);
        let out = sent(&r);
        assert_eq!(out.len(), 2, "and the firing follows");
        assert!(out[1].1.contains("call the bank"), "{}", out[1].1);
    }

    #[test]
    fn a_watch_command_never_reaches_an_agent() {
        // The confirmation has to come from the daemon. Handing "remind me…" to a model would
        // produce a cheerful "will do!" and no alarm — the worst outcome available here.
        let mut r = rig("chat-noagent");
        say(&mut r, "remind me tomorrow 9am to call the bank");
        say(&mut r, "watching");
        say(&mut r, "cancel w1");
        assert!(
            r.opened.lock().unwrap().is_empty(),
            "no session was opened for a watch command"
        );
        assert!(r.watch.live().is_empty(), "and the cancel took effect");
        let out = sent(&r);
        assert!(out[1].1.contains("call the bank"), "listing: {}", out[1].1);
        assert!(out[2].1.contains("cancelled"), "{}", out[2].1);
    }

    #[test]
    fn an_ordinary_task_with_a_time_in_it_still_goes_to_an_agent() {
        let mut r = rig("chat-task");
        say(&mut r, "book me a flight tomorrow");
        assert_eq!(
            r.opened.lock().unwrap().as_slice(),
            ["channel:test:me"],
            "the task opened a session"
        );
        assert!(r.watch.live().is_empty(), "and set no alarm");
    }
}
