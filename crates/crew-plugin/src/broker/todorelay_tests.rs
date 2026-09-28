//! The task's checklist (`sys:todo`) in the relay: it heads every follow-up
//! prompt above the tool exchanges, where shortening them never reaches it,
//! and an answer that left steps on it says which. A turn that never writes
//! one is sent, and answers, exactly what it was before lists existed.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::Registry;
use crew_hive::tools::todo::Checklist;

/// Replies in order, then `@done`; keeps every prompt it was dialed with.
struct Scripted(Mutex<Vec<String>>, Arc<Mutex<Vec<String>>>);

impl Adapter for Scripted {
    fn name(&self) -> &str {
        "coder"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "@done".into()))
    }
}

/// What the session's surface does: `sys:todo` writes the task's list (when
/// it keeps one), and every read answers [`file`].
struct Surface(Option<Checklist>);

impl ToolRunner for Surface {
    fn hint(&self) -> String {
        "TOOLS: sys:todo, sys:read_file".into()
    }
    fn call(&self, _server: &str, tool: &str, args: &str) -> Result<String, String> {
        match (tool, &self.0) {
            ("todo", Some(list)) => list.write(args),
            _ => Ok(file(args)),
        }
    }
    fn checklist(&self) -> Option<&Checklist> {
        self.0.as_ref()
    }
}

/// 2,470 chars: well over the 600 that are never shortened.
fn file(args: &str) -> String {
    (0..40)
        .map(|i| format!("{args} line {i:02}: {}", "x".repeat(30)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn todo(said: &str, items: &[(&str, &str)]) -> String {
    let items: Vec<serde_json::Value> = items
        .iter()
        .map(|(text, status)| serde_json::json!({"text": text, "status": status}))
        .collect();
    format!(
        "{said}\n@tool sys:todo {}",
        serde_json::json!({ "items": items })
    )
}

fn read(name: &str) -> String {
    format!("reading {name}\n@tool sys:read_file {{\"path\":\"{name}\"}}")
}

/// Run one turn of `replies` against `surface`: every prompt the agent was
/// dialed with, and the thread's final answer.
fn turn(surface: Surface, replies: Vec<String>) -> (Vec<String>, String) {
    let dialed = Arc::new(Mutex::new(Vec::new()));
    let mut replies = replies;
    replies.reverse();
    let agent: Box<dyn Adapter> = Box::new(Scripted(Mutex::new(replies), Arc::clone(&dialed)));
    let b = Broker::new(Registry::new(vec![agent]), 2, Duration::from_secs(5))
        .with_tools(Arc::new(surface))
        .with_tool_rounds(8);
    let mut answer = String::new();
    b.run(
        "user",
        "coder",
        "add a /foo command with tests and docs",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |hop| {
            if hop.kind == HopKind::Done {
                answer = hop.text;
            }
        },
    );
    let dialed = dialed.lock().unwrap().clone();
    (dialed, answer)
}

const STEPS: [&str; 3] = ["add the command", "add a test", "write the docs"];

#[test]
fn the_list_heads_each_follow_up_and_the_answer_names_what_was_left() {
    let (dialed, answer) = turn(
        Surface(Some(Checklist::default())),
        vec![
            todo(
                "three steps",
                &[
                    (STEPS[0], "in_progress"),
                    (STEPS[1], "pending"),
                    (STEPS[2], "pending"),
                ],
            ),
            read("big.rs"),
            todo(
                "the command is in",
                &[
                    (STEPS[0], "done"),
                    (STEPS[1], "in_progress"),
                    (STEPS[2], "pending"),
                ],
            ),
            "the command is in\n@done".into(),
        ],
    );
    assert_eq!(dialed.len(), 4, "{dialed:#?}");
    assert!(
        !dialed[0].contains("YOUR CHECKLIST:"),
        "nothing written yet"
    );
    let heads = |list: &str| format!("YOUR CHECKLIST:\n{list}\n\nTOOL EXCHANGES THIS TURN:\n");
    let first =
        "\u{25b6} add the command\n\u{2610} add a test\n\u{2610} write the docs\n0 of 3 done";
    let then =
        "\u{2611} add the command\n\u{25b6} add a test\n\u{2610} write the docs\n1 of 3 done";
    // After the read (round 2): the list as round 1 wrote it, above the log.
    assert!(dialed[2].contains(&heads(first)), "{}", dialed[2]);
    // After the tick (round 3): the new list, once, above the log.
    assert!(dialed[3].contains(&heads(then)), "{}", dialed[3]);
    assert_eq!(dialed[3].matches("YOUR CHECKLIST:").count(), 1);
    assert_eq!(
        answer,
        "the command is in\n\nchecklist: 1 of 3 done \u{2014} not done: add a test, write the docs"
    );
}

#[test]
fn the_list_is_whole_above_exchanges_that_were_shortened() {
    // Long steps, so the list's own exchange is shortened once it is old.
    let steps: Vec<String> = (1..=6)
        .map(|n| format!("step {n} {}", "s".repeat(100)))
        .collect();
    let items: Vec<(&str, &str)> = steps.iter().map(|s| (s.as_str(), "pending")).collect();
    let (dialed, _) = turn(
        Surface(Some(Checklist::default())),
        vec![
            todo("six steps", &items),
            read("a.rs"),
            read("b.rs"),
            read("c.rs"),
            "done\n@done".into(),
        ],
    );
    let last = &dialed[4];
    let (list_at, log_at) = (
        last.find("YOUR CHECKLIST:"),
        last.find("TOOL EXCHANGES THIS TURN:"),
    );
    assert!(list_at.is_some() && list_at < log_at, "{last}");
    let log = &last[log_at.unwrap()..];
    assert!(
        log.matches("(result shortened").count() >= 2,
        "the list's exchange and a.rs are shortened:\n{log}"
    );
    for s in &steps {
        assert!(
            last[..log_at.unwrap()].contains(&format!("\u{2610} {s}\n")),
            "{s} lost"
        );
    }
    assert!(
        last.contains("0 of 6 done\n\nTOOL EXCHANGES THIS TURN:\n"),
        "{last}"
    );
}

#[test]
fn a_turn_that_keeps_no_list_is_sent_and_answers_as_before() {
    let script = || vec![read("a.rs"), read("b.rs"), "answer\n@done".to_string()];
    let (kept, kept_answer) = turn(Surface(Some(Checklist::default())), script());
    let (bare, bare_answer) = turn(Surface(None), script());
    assert_eq!(kept, bare, "a surface that keeps lists changed the prompts");
    assert_eq!(kept_answer, "answer");
    assert_eq!(kept_answer, bare_answer);
    let exchange = format!(
        "YOUR MESSAGE:\nreading a.rs\nCALLED sys:read_file {{\"path\":\"a.rs\"}}\nRESULT:\n{}",
        file("{\"path\":\"a.rs\"}")
    );
    assert_eq!(
        kept[1],
        format!(
            "{}\n\nTOOL EXCHANGES THIS TURN:\n{exchange}\n\nYou may make 7 more tool call(s) \
             this turn, or answer now and end the answer as HOW TO REPLY says.",
            kept[0]
        )
    );
}
