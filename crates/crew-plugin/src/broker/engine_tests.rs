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

fn legs(hops: &[Hop]) -> Vec<(String, String, HopKind)> {
    hops.iter()
        .map(|h| (h.from.clone(), h.to.clone(), h.kind))
        .collect()
}

#[test]
fn demo_a_to_b() {
    // claude hands the task to codex, which finishes.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["TO codex: check this"]),
            Fake::scripted("codex", &["DONE: looks good"]),
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
    assert_eq!(hops[1].text, "looks good");
}

#[test]
fn demo_b_to_a_round_trip() {
    // claude -> codex, codex replies back to claude (B->A), claude finishes.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["TO codex: question", "DONE"]),
            Fake::scripted("codex", &["the answer"]),
        ],
        6,
    );
    // The relay is flat: codex's plain reply bounces back to claude (B->A), and
    // claude's DONE is addressed to whoever last messaged it (codex).
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
    // A(claude) -> B(codex) -> C(opencode); C answers B, who relays it back to
    // A, who finishes. The (codex -> claude) leg is the answer returning to A.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["TO codex: relay please", "DONE: shipped"]),
            Fake::scripted(
                "codex",
                &["TO opencode: consult", "TO claude: opencode says 42"],
            ),
            Fake::scripted("opencode", &["here is C's answer"]),
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
    // Two agents that relay forever; the hop limit must stop the thread.
    let hops = drive(
        vec![
            Fake::scripted("claude", &["TO codex: loop"]),
            Fake::scripted("codex", &["TO claude: loop"]),
        ],
        2,
    );
    let last = hops.last().unwrap();
    assert_eq!(last.kind, HopKind::Terminated);
    assert!(last.text.contains("hop limit"));
    // hops 0,1,2 logged, then the guard fires on hop 3.
    assert_eq!(hops.len(), 4);
}

#[test]
fn unknown_agent_errors() {
    let hops = drive(vec![Fake::scripted("codex", &["DONE"])], 6); // no "claude"
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0].kind, HopKind::Error);
    assert!(hops[0].text.contains("unknown agent"));
}

#[test]
fn call_error_is_logged_and_stops() {
    let hops = drive(vec![Fake::failing("claude")], 6);
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0].kind, HopKind::Error);
    assert_eq!(hops[0].text, "boom");
}

#[test]
fn empty_reply_is_an_error() {
    let hops = drive(vec![Fake::scripted("claude", &[""])], 6);
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0].kind, HopKind::Error);
    assert!(hops[0].text.contains("empty"));
}
