//! Each test runs a whole relay turn against a scripted agent, so what is
//! checked is the prompt the agent was really sent and the answer the pane
//! would really show.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::broker::toolcall::ToolRunner;
use crate::broker::{Adapter, Broker, Hop, HopKind, Registry};

/// An agent that answers from a script, repeating its last line, and keeps
/// every prompt it was sent.
struct Script {
    replies: Mutex<Vec<String>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Adapter for Script {
    fn name(&self) -> &str {
        "claude"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.seen.lock().unwrap().push(body.to_string());
        let mut r = self.replies.lock().unwrap();
        Ok(if r.len() > 1 {
            r.remove(0)
        } else {
            r[0].clone()
        })
    }
}

/// Every call succeeds with the same text.
struct AnyTools;

impl ToolRunner for AnyTools {
    fn hint(&self) -> String {
        "TOOLS: sys:read_file".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok("fn clip() {}".into())
    }
}

/// Run one turn; hand back every prompt the agent saw and the answer.
fn turn(replies: &[&str]) -> (Vec<String>, String) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let agent = Script {
        replies: Mutex::new(replies.iter().map(|r| r.to_string()).collect()),
        seen: Arc::clone(&seen),
    };
    let b = Broker::new(
        Registry::new(vec![Box::new(agent)]),
        2,
        Duration::from_secs(1),
    )
    .with_tools(Arc::new(AnyTools));
    let mut hops: Vec<Hop> = Vec::new();
    b.run(
        "user",
        "claude",
        "why does the clip drop the last line?",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |h| hops.push(h),
    );
    let answer = hops
        .iter()
        .find(|h| h.kind == HopKind::Done)
        .map(|h| h.text.clone())
        .unwrap_or_else(|| {
            panic!(
                "no answer: {:?}",
                hops.iter().map(|h| &h.text).collect::<Vec<_>>()
            )
        });
    let seen = seen.lock().unwrap().clone();
    (seen, answer)
}

#[test]
fn the_follow_up_prompt_carries_what_the_agent_wrote_with_its_call() {
    let (seen, answer) = turn(&[
        "The bug is in clip(); reading it next.\n@tool sys:read_file {\"path\":\"a.rs\"}",
        "clip() drops it on purpose.\n@done",
    ]);
    assert_eq!(answer, "clip() drops it on purpose.");
    assert_eq!(seen.len(), 2, "one call, then the answer");
    assert!(
        seen[1].contains("The bug is in clip(); reading it next."),
        "second prompt lost the message:\n{}",
        seen[1]
    );
    assert!(
        seen[1].contains(
            "YOUR MESSAGE:\nThe bug is in clip(); reading it next.\nCALLED sys:read_file"
        ),
        "the message sits above its call:\n{}",
        seen[1]
    );
}

#[test]
fn a_spent_budget_answers_with_the_text_above_a_fenced_call() {
    let text = "clip() cuts at the budget, not at a line end.";
    let reply = format!(
        "{text}\n\n```json\n@tool sys:read_file {{\n  \"path\": \"a.rs\",\n  \
         \"offset\": 5480\n}}\n```\n@done"
    );
    let (_, answer) = turn(&[&reply]);
    assert_eq!(answer, text);
    assert!(!answer.contains("@tool"), "{answer}");
    assert!(!answer.contains("```"), "{answer}");
    assert!(!answer.contains('{'), "{answer}");
}

#[test]
fn a_spent_budget_on_a_bare_call_answers_with_one_plain_line() {
    let (_, answer) = turn(&["@tool sys:read_file {\"path\":\"a.rs\"}"]);
    assert_eq!(
        answer,
        format!(
            "stopped before answering: the tool budget ({} calls) ran out",
            crew_hive::tools::MAX_TOOL_ROUNDS
        )
    );
}

#[test]
fn a_reply_that_was_only_the_call_adds_no_message() {
    let e = exchange("", "sys:read_file", "{}", "fn clip() {}");
    assert_eq!(
        e.to_string(),
        "CALLED sys:read_file {}\nRESULT:\nfn clip() {}"
    );
}

#[test]
fn text_above_the_call_is_the_answer_when_it_is_all_there_is() {
    assert_eq!(budget_answer("  kept  \n", 4), "kept");
}
