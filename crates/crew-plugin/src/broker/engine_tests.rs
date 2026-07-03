use super::*;
use crate::broker::Adapter;
use std::sync::Mutex;

/// A scripted agent: returns its replies in order, repeating the last once
/// exhausted. `fail` makes every call error (to exercise the error path).
struct Fake {
    name: String,
    replies: Vec<String>,
    idx: Mutex<usize>,
    fail: bool,
}

impl Fake {
    fn scripted(name: &str, replies: &[&str]) -> Box<dyn Adapter> {
        Box::new(Fake {
            name: name.into(),
            replies: replies.iter().map(|s| s.to_string()).collect(),
            idx: Mutex::new(0),
            fail: false,
        })
    }
    fn failing(name: &str) -> Box<dyn Adapter> {
        Box::new(Fake {
            name: name.into(),
            replies: vec![],
            idx: Mutex::new(0),
            fail: true,
        })
    }
}

impl Adapter for Fake {
    fn name(&self) -> &str {
        &self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _body: &str, _t: std::time::Duration) -> Result<String, String> {
        if self.fail {
            return Err("boom".into());
        }
        let mut i = self.idx.lock().unwrap();
        let r = self
            .replies
            .get(*i)
            .or_else(|| self.replies.last())
            .cloned()
            .unwrap_or_default();
        *i += 1;
        Ok(r)
    }
}

/// A scripted agent that also records every prompt it was called with, so a
/// test can inspect what a LATER hop's prompt contained (e.g. whether the
/// transcript grew a line for an earlier, empty-bodied relay).
struct Capturing {
    name: String,
    replies: Vec<String>,
    idx: Mutex<usize>,
    calls: std::sync::Arc<Mutex<Vec<String>>>,
}

impl Capturing {
    fn scripted(
        name: &str,
        replies: &[&str],
    ) -> (Box<dyn Adapter>, std::sync::Arc<Mutex<Vec<String>>>) {
        let calls = std::sync::Arc::new(Mutex::new(Vec::new()));
        let agent = Box::new(Capturing {
            name: name.into(),
            replies: replies.iter().map(|s| s.to_string()).collect(),
            idx: Mutex::new(0),
            calls: std::sync::Arc::clone(&calls),
        });
        (agent, calls)
    }
}

impl Adapter for Capturing {
    fn name(&self) -> &str {
        &self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.calls.lock().unwrap().push(body.to_string());
        let mut i = self.idx.lock().unwrap();
        let r = self
            .replies
            .get(*i)
            .or_else(|| self.replies.last())
            .cloned()
            .unwrap_or_default();
        *i += 1;
        Ok(r)
    }
}

fn drive(agents: Vec<Box<dyn Adapter>>, max: u32) -> Vec<Hop> {
    let b = Broker::new(
        Registry::new(agents),
        max,
        std::time::Duration::from_secs(1),
    );
    let mut hops = Vec::new();
    b.run("user", "claude", "task", "t1", &mut |h| hops.push(h));
    hops
}

/// Routing legs, ignoring the `Dialing` progress notes emitted before calls.
fn legs(hops: &[Hop]) -> Vec<(String, String, HopKind)> {
    hops.iter()
        .filter(|h| h.kind != HopKind::Dialing)
        .map(|h| (h.from.clone(), h.to.clone(), h.kind))
        .collect()
}

fn errors(hops: &[Hop]) -> Vec<&Hop> {
    hops.iter().filter(|h| h.kind == HopKind::Error).collect()
}

#[test]
fn demo_a_to_b() {
    // claude hands the task to codex via @next, which finishes with @done.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["check this\n@next codex"]),
            Fake::scripted("codex", &["looks good\n@done"]),
        ],
        6,
    );
    assert_eq!(
        legs(&hops),
        vec![
            ("claude".into(), "codex".into(), HopKind::Reply),
            ("codex".into(), "claude".into(), HopKind::Done),
        ]
    );
    // Control line stripped (relayed body is just the answer); done text kept.
    let txt = |k| hops.iter().find(|h| h.kind == k).map(|h| h.text.clone());
    assert_eq!(txt(HopKind::Reply).as_deref(), Some("check this"));
    assert_eq!(txt(HopKind::Done).as_deref(), Some("looks good"));
    // Each call is announced first (UI shows activity during the wait).
    let dialed: Vec<&str> = hops
        .iter()
        .filter(|h| h.kind == HopKind::Dialing)
        .map(|h| h.to.as_str())
        .collect();
    assert_eq!(dialed, vec!["claude", "codex"]);
}

#[test]
fn demo_b_to_a_round_trip() {
    // claude -> codex, codex relays back to claude (B->A), claude finishes.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["question\n@next codex", "@done"]),
            Fake::scripted("codex", &["the answer\n@next claude"]),
        ],
        6,
    );
    assert_eq!(
        legs(&hops),
        vec![
            ("claude".into(), "codex".into(), HopKind::Reply),
            ("codex".into(), "claude".into(), HopKind::Reply),
            ("claude".into(), "codex".into(), HopKind::Done),
        ]
    );
}

#[test]
fn demo_three_way_relay_answer_returns_to_a() {
    // A->B->C; C relays its answer back B->A, who finishes.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["relay\n@next codex", "shipped\n@done"]),
            Fake::scripted(
                "codex",
                &["consult\n@next opencode", "C says 42\n@next claude"],
            ),
            Fake::scripted("opencode", &["here is C answer\n@next codex"]),
        ],
        6,
    );
    assert_eq!(
        legs(&hops),
        vec![
            ("claude".into(), "codex".into(), HopKind::Reply),
            ("codex".into(), "opencode".into(), HopKind::Reply),
            ("opencode".into(), "codex".into(), HopKind::Reply),
            ("codex".into(), "claude".into(), HopKind::Reply),
            ("claude".into(), "codex".into(), HopKind::Done),
        ]
    );
}

#[test]
fn loop_guard_terminates_a_cycle() {
    // Relay forever with distinct bodies → exercises the hop limit guard.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["a\n@next codex", "c\n@next codex"]),
            Fake::scripted("codex", &["b\n@next claude"]),
        ],
        2,
    );
    let last = hops.last().unwrap();
    assert_eq!(last.kind, HopKind::Terminated);
    assert!(last.text.contains("hop limit"));
    assert_eq!(legs(&hops).len(), 4); // 3 relays logged, then the guard fires.
}

#[test]
fn missing_directive_ends_the_thread() {
    let hops = drive(vec![Fake::scripted("claude", &["answer no directive"])], 6);
    assert_eq!(legs(&hops).len(), 1);
    assert_eq!(legs(&hops)[0].2, HopKind::Done);
}

#[test]
fn unknown_agent_errors() {
    let hops = drive(vec![Fake::scripted("codex", &["@done"])], 6); // no "claude"
    let errs = errors(&hops);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].text.contains("unknown agent"));
}

#[test]
fn call_error_is_logged_and_stops() {
    let hops = drive(vec![Fake::failing("claude")], 6);
    let errs = errors(&hops);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].text, "boom");
}

#[test]
fn empty_reply_is_an_error() {
    let hops = drive(vec![Fake::scripted("claude", &[""])], 6);
    let errs = errors(&hops);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].text.contains("empty"));
}

#[test]
fn keep_in_transcript_skips_blank_bodies_only() {
    assert!(!keep_in_transcript(""));
    assert!(!keep_in_transcript("   \n\t"));
    assert!(keep_in_transcript("ok"));
    assert!(keep_in_transcript("  ok  "));
}

#[test]
fn empty_relay_body_does_not_bloat_a_later_prompt() {
    // claude hands off to codex with a BLANK body (just the control line);
    // codex then hands back to claude with a real answer. If the blank hop
    // still cost a transcript line, codex's prompt would carry a
    // "claude → codex: " entry with no information in it.
    let (claude, _claude_calls) = Capturing::scripted("claude", &["\n@next codex", "final\n@done"]);
    let (codex, codex_calls) = Capturing::scripted("codex", &["ack\n@next claude"]);
    let b = Broker::new(
        Registry::new(vec![claude, codex]),
        6,
        std::time::Duration::from_secs(1),
    );
    let mut hops = Vec::new();
    b.run("user", "claude", "task", "t1", &mut |h| hops.push(h));
    let calls = codex_calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(
        !calls[0].contains("claude \u{2192} codex:"),
        "empty-bodied hop leaked into the transcript: {}",
        calls[0]
    );
    assert!(
        calls[0].contains("you are first"),
        "with nothing kept, codex should see the no-transcript-yet placeholder: {}",
        calls[0]
    );
}

#[test]
fn is_dup_flags_only_a_byte_identical_immediate_repeat() {
    let none: Vec<String> = Vec::new();
    assert!(!is_dup(&none, "a"));
    let one = vec!["a".to_string()];
    assert!(is_dup(&one, "a"));
    assert!(!is_dup(&one, "b"));
    let two = vec!["a".to_string(), "b".to_string()];
    assert!(
        !is_dup(&two, "a"),
        "only the immediately-preceding entry counts"
    );
    assert!(is_dup(&two, "b"));
}

#[test]
fn consecutive_duplicate_relay_body_is_not_logged_twice() {
    // claude relays "X" to codex; codex hands straight back with a BLANK body
    // (so the no-progress guard's `last_body` check doesn't fire); claude then
    // relays "X" again. The two "claude → codex: X" transcript entries are
    // separated only by the blank (unlogged) hop, so they'd land back-to-back
    // in the transcript — a byte-identical consecutive repeat that must be
    // deduped rather than doubling the next prompt's token cost.
    let (claude, _) = Capturing::scripted(
        "claude",
        &["X\n@next codex", "X\n@next codex", "final\n@done"],
    );
    let (codex, codex_calls) =
        Capturing::scripted("codex", &["\n@next claude", "ack\n@next claude"]);
    let b = Broker::new(
        Registry::new(vec![claude, codex]),
        6,
        std::time::Duration::from_secs(1),
    );
    let mut hops = Vec::new();
    b.run("user", "claude", "task", "t1", &mut |h| hops.push(h));
    let calls = codex_calls.lock().unwrap();
    assert_eq!(calls.len(), 2, "{calls:?}");
    let count = calls[1].matches("claude \u{2192} codex: X").count();
    assert_eq!(
        count, 1,
        "duplicate consecutive entry must not repeat: {}",
        calls[1]
    );
}

#[test]
fn non_empty_relay_body_is_kept_in_the_transcript() {
    let (claude, _) = Capturing::scripted("claude", &["real answer\n@next codex", "final\n@done"]);
    let (codex, codex_calls) = Capturing::scripted("codex", &["ack\n@next claude"]);
    let b = Broker::new(
        Registry::new(vec![claude, codex]),
        6,
        std::time::Duration::from_secs(1),
    );
    let mut hops = Vec::new();
    b.run("user", "claude", "task", "t1", &mut |h| hops.push(h));
    let calls = codex_calls.lock().unwrap();
    assert!(
        calls[0].contains("claude \u{2192} codex: real answer"),
        "{}",
        calls[0]
    );
}
