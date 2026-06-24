//! End-to-end relay scenarios driven through the real `crew-broker-plugin`
//! binary with fake agents on PATH. Proves A→B, B→A, the 3-way relay (with a
//! real JSON-normalized opencode reply in the loop), and the loop guard — all
//! using the structured `@next`/`@done` protocol.
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
    // claude hands off to codex (@next), which finishes (@done).
    let dir = unique_dir("a2b");
    write_fake(&dir, "claude", &["please pong\\n@next codex"], false);
    write_fake(&dir, "codex", &["pong-ok\\n@done"], false);
    write_fake(&dir, "opencode", &["@done"], false);

    let ev = run_broker(&dir, &[], &[SEND]);
    let msgs = messages(&ev);
    assert!(has_leg(&ev, "claude → codex"), "{msgs:?}");
    assert!(has_leg(&ev, "codex → claude"), "{msgs:?}");
    // Control line stripped: the relayed body is just the answer.
    assert_eq!(text_of(&msgs, "claude → codex"), "please pong");
    assert_eq!(text_of(&msgs, "codex → claude"), "[done] pong-ok");
    // A cost summary is surfaced at the end.
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("tokens")),
        "{msgs:?}"
    );
}

#[test]
fn round_trip_b_to_a() {
    // codex relays back to claude (B→A) via @next; claude then finishes.
    let dir = unique_dir("b2a");
    write_fake(&dir, "claude", &["question\\n@next codex", "@done"], false);
    write_fake(&dir, "codex", &["the answer is 42\\n@next claude"], false);
    write_fake(&dir, "opencode", &["@done"], false);

    let ev = run_broker(&dir, &[], &[SEND]);
    let msgs = messages(&ev);
    assert!(has_leg(&ev, "codex → claude"), "{msgs:?}");
    assert_eq!(text_of(&msgs, "codex → claude"), "the answer is 42");
}

#[test]
fn three_way_relay_with_opencode_json() {
    // claude → codex → opencode; opencode's JSON reply (with its @next line) is
    // normalized and relayed back to codex, who relays to claude, who finishes.
    let dir = unique_dir("3way");
    write_fake(
        &dir,
        "claude",
        &["relay\\n@next codex", "shipped\\n@done"],
        false,
    );
    write_fake(
        &dir,
        "codex",
        &["consult\\n@next opencode", "oc says 42\\n@next claude"],
        false,
    );
    write_fake(&dir, "opencode", &["here is C answer\\n@next codex"], true); // JSON

    let ev = run_broker(&dir, &[], &[SEND]);
    let msgs = messages(&ev);
    assert!(has_leg(&ev, "codex → opencode"), "{msgs:?}");
    // opencode's JSON reply was normalized AND its @next parsed → relayed to codex.
    assert!(has_leg(&ev, "opencode → codex"), "{msgs:?}");
    assert_eq!(text_of(&msgs, "opencode → codex"), "here is C answer");
    // ...and the answer relays back toward A (claude).
    assert!(has_leg(&ev, "codex → claude"), "{msgs:?}");
}

#[test]
fn loop_guard_terminates_via_binary() {
    // Two agents that relay forever; the hop limit forced to 2 stops the thread.
    let dir = unique_dir("loop");
    // The e2e fake finishes once its scripted replies run out, so give claude
    // enough @next turns to keep the cycle going until the hop guard fires.
    // Distinct bodies so this hits the hop guard, not the no-progress guard.
    write_fake(
        &dir,
        "claude",
        &["a\\n@next codex", "c\\n@next codex"],
        false,
    );
    write_fake(&dir, "codex", &["b\\n@next claude"], false);

    let ev = run_broker(&dir, &[("CREW_BROKER_MAX_HOPS", "2")], &[SEND]);
    let msgs = messages(&ev);
    // The guard fires (a cost summary follows it, so check anywhere, not last).
    assert!(
        msgs.iter()
            .any(|(_, t)| t.contains("[stopped]") && t.contains("hop limit")),
        "{msgs:?}"
    );
}
