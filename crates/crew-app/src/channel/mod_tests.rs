//! Routing is the part that goes wrong quietly: a reply sent to the wrong place, or to nowhere,
//! looks identical to a reply that was never written. So most of this is about delivery.
use super::loopback::Loopback;
use super::*;

fn router_with(
    kinds: &[&str],
) -> (
    Router,
    Vec<std::sync::Arc<std::sync::Mutex<loopback::Wire>>>,
) {
    let mut r = Router::new();
    let mut wires = Vec::new();
    for k in kinds {
        let (c, w) = Loopback::pair(k);
        r.add(Box::new(c)).expect("register");
        wires.push(w);
    }
    (r, wires)
}

#[test]
fn an_address_splits_into_a_kind_and_a_rest() {
    assert_eq!(split_address("telegram:12345"), Some(("telegram", "12345")));
    // The rest may itself contain colons — a channel's addressing is its own business.
    assert_eq!(
        split_address("voice:mic:default"),
        Some(("voice", "mic:default"))
    );
}

/// A bare word is not an address. Guessing a default channel here would send someone's reply to
/// a stranger on whichever channel happened to be registered first.
#[test]
fn an_address_without_a_kind_is_not_routable() {
    assert_eq!(split_address("12345"), None);
    assert_eq!(split_address(""), None);
    assert_eq!(split_address(":12345"), None, "an empty kind names nothing");
    assert_eq!(
        split_address("telegram:"),
        None,
        "an empty rest names nobody"
    );
}

#[test]
fn a_reply_reaches_the_channel_its_address_names_and_no_other() {
    let (mut r, wires) = router_with(&["telegram", "voice"]);
    r.send("telegram:me", "hello").expect("routed");
    let tg = wires[0].lock().unwrap();
    let voice = wires[1].lock().unwrap();
    assert_eq!(
        tg.outbox,
        vec![("telegram:me".to_string(), "hello".to_string())]
    );
    assert!(voice.outbox.is_empty(), "the other channel heard nothing");
}

/// The failure that matters: an undeliverable reply must be an error the caller can act on, not
/// a message that vanishes.
#[test]
fn an_unroutable_address_is_an_error_not_a_silent_drop() {
    let (mut r, _w) = router_with(&["telegram"]);
    let err = r.send("sms:12345", "hello").unwrap_err();
    assert!(
        err.contains("sms"),
        "the error names the missing channel: {err}"
    );
    let err = r.send("nonsense", "hello").unwrap_err();
    assert!(
        err.contains("kind:rest"),
        "the error explains the shape: {err}"
    );
}

/// Two channels claiming `telegram:` would make every reply a coin flip about which delivers it.
#[test]
fn two_channels_cannot_own_the_same_address_kind() {
    let mut r = Router::new();
    let (a, _wa) = Loopback::pair("telegram");
    let (b, _wb) = Loopback::pair("telegram");
    r.add(Box::new(a)).expect("the first one registers");
    let err = r.add(Box::new(b)).unwrap_err();
    assert!(err.contains("telegram"), "{err}");
    assert_eq!(r.kinds(), vec!["telegram"], "only one owner survives");
}

/// A kind containing a colon would make its own addresses unsplittable.
#[test]
fn a_channel_kind_cannot_contain_a_colon() {
    let mut r = Router::new();
    let (bad, _w) = Loopback::pair("tele:gram");
    assert!(r.add(Box::new(bad)).is_err());
}

/// Polling must keep each message's origin, or nobody can answer it.
#[test]
fn polled_messages_keep_the_address_to_answer() {
    let (mut r, wires) = router_with(&["telegram", "voice"]);
    wires[0].lock().unwrap().inbox.push(Inbound {
        from: "telegram:me".into(),
        text: "what is on my calendar".into(),
    });
    wires[1].lock().unwrap().inbox.push(Inbound {
        from: "voice:kitchen".into(),
        text: "set a timer".into(),
    });
    let got = r.poll();
    assert_eq!(got.len(), 2);
    let addrs: Vec<&str> = got.iter().map(|i| i.from.as_str()).collect();
    assert!(addrs.contains(&"telegram:me") && addrs.contains(&"voice:kitchen"));
    // And the round trip: every polled message can be answered where it came from.
    for i in &got {
        r.send(&i.from, "ok").expect("every origin is routable");
    }
    assert_eq!(wires[0].lock().unwrap().outbox.len(), 1);
    assert_eq!(wires[1].lock().unwrap().outbox.len(), 1);
}

#[test]
fn polling_drains_so_a_message_is_delivered_once() {
    let (mut r, wires) = router_with(&["telegram"]);
    wires[0].lock().unwrap().inbox.push(Inbound {
        from: "telegram:me".into(),
        text: "once".into(),
    });
    assert_eq!(r.poll().len(), 1);
    assert!(
        r.poll().is_empty(),
        "a message is not replayed on the next poll"
    );
}

/// A channel that exists but has no credential is present and inert. Sending through it must
/// fail loudly rather than appear to succeed — this is the shape every real channel starts in,
/// before its token is configured.
#[test]
fn an_unconfigured_channel_is_listed_but_refuses_to_send() {
    let (mut r, wires) = router_with(&["telegram"]);
    wires[0].lock().unwrap().ready = false;
    assert_eq!(r.kinds(), vec!["telegram"], "still registered");
    assert!(r.ready_kinds().is_empty(), "but not usable");
    let err = r.send("telegram:me", "hello").unwrap_err();
    assert!(err.contains("not configured"), "{err}");
    assert!(
        wires[0].lock().unwrap().outbox.is_empty(),
        "nothing was handed to an unusable transport"
    );
}
