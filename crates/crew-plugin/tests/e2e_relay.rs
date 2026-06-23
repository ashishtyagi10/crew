//! End-to-end relay scenarios driven through the real `crew-broker-plugin`
//! binary with fake agents on PATH. Proves A→B, B→A, the 3-way relay (with a
//! real JSON-normalized opencode reply in the loop), and the loop guard.
mod common;
use common::{has_leg, messages, run_broker, unique_dir, write_fake};

const SEND: &str = r#"{"type":"send","channel":"crew","text":"do it"}"#;

fn text_of<'a>(msgs: &'a [(String, String)], sender: &str) -> &'a str {
    msgs.iter()
        .find(|(s, _)| s == sender)
        .map(|(_, t)| t.as_str())
        .unwrap_or("")
}

#[test]
fn relay_a_to_b() {
    // claude hands off to codex, which finishes the thread.
    let dir = unique_dir("a2b");
    write_fake(&dir, "claude", &["TO codex: please pong"], false);
    write_fake(&dir, "codex", &["DONE: pong-ok"], false);
    write_fake(&dir, "opencode", &["DONE"], false);

    let ev = run_broker(&dir, &[], &[SEND]);
    assert!(has_leg(&ev, "claude → codex"), "{:?}", messages(&ev));
    assert!(has_leg(&ev, "codex → claude"), "{:?}", messages(&ev));
    let msgs = messages(&ev);
    assert_eq!(text_of(&msgs, "claude → codex"), "TO codex: please pong");
    assert_eq!(text_of(&msgs, "codex → claude"), "[done] pong-ok");
}

#[test]
fn round_trip_b_to_a() {
    // codex's plain reply bounces back to claude (B→A), which then finishes.
    let dir = unique_dir("b2a");
    write_fake(
        &dir,
        "claude",
        &["TO codex: question", "DONE: wrapped"],
        false,
    );
    write_fake(&dir, "codex", &["the answer is 42"], false);
    write_fake(&dir, "opencode", &["DONE"], false);

    let ev = run_broker(&dir, &[], &[SEND]);
    let msgs = messages(&ev);
    assert!(has_leg(&ev, "codex → claude"), "{msgs:?}");
    assert_eq!(text_of(&msgs, "codex → claude"), "the answer is 42");
}

#[test]
fn three_way_relay_with_opencode_json() {
    // claude → codex → opencode; opencode's JSON reply is normalized and bounced
    // back, codex relays it to claude, claude finishes. Exercises the JSON
    // normalizer inside a live relay.
    let dir = unique_dir("3way");
    write_fake(&dir, "claude", &["TO codex: relay", "DONE: shipped"], false);
    write_fake(
        &dir,
        "codex",
        &["TO opencode: consult", "TO claude: oc says 42"],
        false,
    );
    write_fake(&dir, "opencode", &["here is C answer"], true); // JSON-wrapped

    let ev = run_broker(&dir, &[], &[SEND]);
    let msgs = messages(&ev);
    assert!(has_leg(&ev, "codex → opencode"), "{msgs:?}");
    // opencode's JSON ({"type":"text","text":"here is C answer"}) was normalized
    // to plain text and bounced back to codex:
    assert!(has_leg(&ev, "opencode → codex"), "{msgs:?}");
    assert_eq!(text_of(&msgs, "opencode → codex"), "here is C answer");
    // ...and the answer is relayed back toward A (claude):
    assert!(has_leg(&ev, "codex → claude"), "{msgs:?}");
}

#[test]
fn loop_guard_terminates_via_binary() {
    // Two agents that relay forever; with the hop limit forced to 2 the broker
    // drops the thread and logs that it stopped.
    let dir = unique_dir("loop");
    write_fake(&dir, "claude", &["TO codex: loop", "TO codex: loop"], false);
    write_fake(&dir, "codex", &["TO claude: loop"], false);

    let ev = run_broker(&dir, &[("CREW_BROKER_MAX_HOPS", "2")], &[SEND]);
    let msgs = messages(&ev);
    let last = &msgs.last().unwrap().1;
    assert!(last.contains("[stopped]"), "{msgs:?}");
    assert!(last.contains("hop limit"), "{msgs:?}");
}
