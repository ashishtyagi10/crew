//! An agent is told each thing once: its role, one rule on length, and one
//! way to finish, which its tool follow-ups point back to, not restate.
//!
//! Captured live before this: a system prompt saying "Be concise." over a
//! frame saying "answer in full", "a CLI coding agent" under a persona whose
//! specialty was something else, and a tool follow-up offering `@next
//! <agent>` to an agent the frame had just told the turn was its alone.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::ApiAdapter;
use crate::broker::adapter::Adapter;
use crate::broker::route::{frame, intro_of};
use crate::broker::tick::noop_tick_emit;
use crate::broker::toolcall::ToolRunner;
use crate::broker::Envelope;
use crate::{Broker, Registry};

/// A CLI-style agent (no system prompt of its own) with scripted replies;
/// records every body it is dialed with.
struct Scripted {
    name: &'static str,
    replies: Mutex<Vec<&'static str>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Adapter for Scripted {
    fn name(&self) -> &str {
        self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.seen.lock().unwrap().push(body.to_string());
        let mut r = self.replies.lock().unwrap();
        Ok(if r.len() > 1 { r.remove(0) } else { r[0] }.to_string())
    }
}

fn scripted(
    name: &'static str,
    replies: &[&'static str],
) -> (Box<dyn Adapter>, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let a = Scripted {
        name,
        replies: Mutex::new(replies.to_vec()),
        seen: seen.clone(),
    };
    (Box::new(a), seen)
}

struct ReadTool;

impl ToolRunner for ReadTool {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok("const TASK_CAP: usize = 4000;".into())
    }
}

/// One relay turn started at `first`, with a tool that can be read.
fn turn(agents: Vec<Box<dyn Adapter>>, first: &str) {
    let b = Broker::new(Registry::new(agents), 6, Duration::from_secs(5))
        .with_tools(Arc::new(ReadTool));
    b.run(
        "user",
        first,
        "where is TASK_CAP?",
        "t",
        &noop_tick_emit(),
        &mut |_| {},
    );
}

const READ_THEN_ANSWER: &[&str] = &[
    "reading\n@tool fs:read {\"path\": \"route.rs\"}",
    "route.rs\n@done",
];

#[test]
fn alone_neither_the_frame_nor_its_tool_follow_up_offers_a_hand_off() {
    let (a, seen) = scripted("coder", READ_THEN_ANSWER);
    turn(vec![a], "coder");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "the frame, then one follow-up: {seen:?}");
    let (first, follow) = (&seen[0], &seen[1]);
    assert!(first.contains("this turn is yours alone"), "{first}");
    assert!(
        !first.contains("@next"),
        "the alone frame offered a hand-off:\n{first}"
    );
    assert!(
        !follow.contains("@next"),
        "the alone follow-up offered a hand-off:\n{follow}"
    );
    assert!(follow.ends_with("as HOW TO REPLY says."), "{follow}");
}

#[test]
fn with_peers_the_frame_and_its_follow_up_still_offer_the_hand_off() {
    let (a, seen) = scripted("coder", READ_THEN_ANSWER);
    let (b, _) = scripted("reviewer", &["@done"]);
    turn(vec![a, b], "coder");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "{seen:?}");
    for p in seen.iter() {
        assert!(p.contains("`@next <agent>`"), "{p}");
        assert!(p.contains("(only from: reviewer)"), "{p}");
    }
}

/// The relay prompt as an API specialist receives it: its system prompt,
/// then the frame the engine builds for it, alone on the turn.
fn assembled(a: &ApiAdapter, body: &str) -> String {
    let env = Envelope::new("user", a.name(), "t", body);
    let system = a.system.clone().expect("a specialist has a persona");
    format!(
        "{system}\n\n{}",
        frame(&env, intro_of(a), &[], body, "", "")
    )
}

fn specialist(name: &str, role: &str) -> ApiAdapter {
    let p: Arc<dyn crew_hive::Provider> = Arc::new(crew_hive::MockProvider {
        reply: String::new(),
    });
    ApiAdapter::specialist(name, role, "m", p).unwrap()
}

#[test]
fn an_agent_alone_gets_one_rule_on_length() {
    let whole = assembled(
        &specialist("quality-assurer", "quality, testing, reporting"),
        "go",
    );
    assert!(!whole.contains("Be concise"), "{whole}");
    assert!(!whole.to_lowercase().contains("concise"), "{whole}");
    assert_eq!(whole.matches("answer in full").count(), 1, "{whole}");
    // Nor is "do the work" said twice (persona, then frame).
    assert_eq!(
        whole.to_lowercase().matches("do the work").count(),
        1,
        "{whole}"
    );
}

#[test]
fn a_specialist_is_introduced_once_by_its_role_never_as_a_cli_coding_agent() {
    let editor = specialist("editor", "proofreading, editing, publication");
    let whole = assembled(&editor, "tidy the README");
    assert!(
        whole.starts_with(
            "You are the editor. Your specialty is proofreading, editing, publication."
        ),
        "{whole}"
    );
    assert!(!whole.contains("CLI coding agent"), "{whole}");
    assert_eq!(
        whole.matches("proofreading, editing, publication").count(),
        1,
        "{whole}"
    );
    // The frame names the agent's handle and how it works, not its role again.
    assert!(
        whole.contains("\n\nYou are \"editor\", a CLI agent.\n"),
        "{whole}"
    );
}

#[test]
fn an_agent_with_no_persona_of_its_own_is_introduced_by_its_role() {
    let (claude, _) = scripted("claude", &["@done"]);
    assert_eq!(intro_of(claude.as_ref()), Some("planning, analysis, prose"));
    let env = Envelope::new("user", "claude", "t", "go");
    let p = frame(&env, intro_of(claude.as_ref()), &[], "go", "", "");
    assert!(
        p.starts_with(
            "You are \"claude\", a CLI agent whose specialty is planning, analysis, prose.\n"
        ),
        "{p}"
    );
    assert!(!p.contains("CLI coding agent"), "{p}");
    // No role to name: a CLI agent, still not a coding one.
    let (x, _) = scripted("x", &["@done"]);
    let env = Envelope::new("user", "x", "t", "go");
    let p = frame(&env, intro_of(x.as_ref()), &[], "go", "", "");
    assert!(p.starts_with("You are \"x\", a CLI agent.\n"), "{p}");
}
