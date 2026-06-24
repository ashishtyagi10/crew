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
