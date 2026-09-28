//! A relay turn whose follow-up outgrows the model's context: cut once and
//! dialed again, and only for that error. Each test runs the turn's tool loop
//! against a scripted agent with a window and reads what it was really sent.
use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::broker::hop::RunStats;
use crate::broker::tier::{tier_of, Tier};
use crate::broker::toolcall::ToolRunner;
use crate::Registry;
use crew_hive::provider::ProviderError;

const DASHSCOPE: &str = r#"{"error":{"message":"Range of input length should be [1, 30720]","code":"invalid_parameter_error"}}"#;
const BAD_INPUT: &str = r#"{"error":{"message":"<400> InternalError.Algo.InvalidParameter: An unknown error occurred due to an unsupported input format.","code":"invalid_parameter_error"}}"#;

/// The window, in chars: the base and two reads fit, the third's follow-up
/// (the first read shortened, `b.rs` and `c.rs` whole) does not.
const N: usize = 8_000;

const RECALLED: &str = "From earlier work in this project:\n- 3d ago \u{2014} you asked: \
                        why is the router slow\n- files that came up: src/route.rs";

/// The base every follow-up starts with: a card, a recalled block, the task.
fn base() -> String {
    format!(
        "PROJECT CARD: crew, a GPU terminal\n\nPROJECT INSTRUCTIONS: keep files short\n\n\
         TASK:\n{RECALLED}\n\nwhat is in a.rs, b.rs and c.rs?"
    )
}

/// Refuses, as an API agent words it, a dial over [`N`] chars and every dial
/// from the `from`-th (from 0); otherwise the next reply. Keeps every dial.
struct Window {
    replies: Mutex<Vec<String>>,
    dialed: Mutex<Vec<String>>,
    refusal: &'static str,
    from: Option<usize>,
}

impl Adapter for Window {
    fn name(&self) -> &str {
        "smith"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        let mut dialed = self.dialed.lock().unwrap();
        let over = body.chars().count() > N || self.from.is_some_and(|f| dialed.len() >= f);
        dialed.push(body.to_string());
        match over {
            true => Err(ProviderError::Api(self.refusal.into()).to_string()),
            false => Ok(self.replies.lock().unwrap().pop().unwrap_or_default()),
        }
    }
}

/// `sys:read_file` answering [`file`]; keeps the paths it ran.
struct Files(Mutex<Vec<String>>);

impl ToolRunner for Files {
    fn hint(&self) -> String {
        "TOOLS: sys:read_file".into()
    }
    fn call(&self, _: &str, _: &str, args: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
        let path = v["path"].as_str().unwrap_or_default().to_string();
        self.0.lock().unwrap().push(path.clone());
        Ok(file(&path))
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        tier_of(server, tool) == Tier::Read
    }
}

/// 5,500 chars for `c.rs`, 3,000 for any other path, in numbered lines.
fn file(path: &str) -> String {
    let size = if path == "c.rs" { 5_500 } else { 3_000 };
    let lines: Vec<String> = (0..size / 50)
        .map(|i| format!("{path} line {i:03}: {}", "q".repeat(34)))
        .collect();
    let s = lines.join("\n");
    format!("{s}{}", " ".repeat(size - s.chars().count()))
}

fn read(path: &str) -> String {
    format!("reading {path}\n@tool sys:read_file {{\"path\":\"{path}\"}}")
}

struct Turn {
    answer: String,
    ran: Vec<String>,
    dialed: Vec<String>,
    notes: usize,
    errors: Vec<String>,
}

/// A turn that reads `a.rs` (the reply the tool loop starts from), then each
/// of `then`, then answers.
fn turn(then: &[&str], refusal: &'static str, from: Option<usize>) -> Turn {
    let runner = std::sync::Arc::new(Files(Mutex::new(Vec::new())));
    let b = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(runner.clone())
        .with_tool_rounds(8);
    let mut replies: Vec<String> = then.iter().map(|p| read(p)).collect();
    replies.push("all read\n@done".into());
    replies.reverse();
    let agent = Window {
        replies: Mutex::new(replies),
        dialed: Mutex::new(Vec::new()),
        refusal,
        from,
    };
    let mut hops = Vec::new();
    let answer = b.run_tools(
        &agent,
        &base(),
        read("a.rs"),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "smith", "t1", "task"),
        &HopStream::noop(),
        &mut |h| hops.push(h),
    );
    let of = |kind| hops.iter().filter(move |h| h.kind == kind).map(|h| &h.text);
    let (ran, dialed) = (
        runner.0.lock().unwrap().clone(),
        agent.dialed.lock().unwrap().clone(),
    );
    Turn {
        answer,
        ran,
        dialed,
        notes: of(HopKind::Reply).filter(|t| *t == CONTEXT_FULL).count(),
        errors: of(HopKind::Error).cloned().collect(),
    }
}

#[test]
fn the_third_follow_up_overflows_is_cut_and_the_turn_answers() {
    let t = turn(&["b.rs", "c.rs"], DASHSCOPE, None);
    assert_eq!(t.answer, "all read\n@done");
    assert_eq!(t.ran, ["a.rs", "b.rs", "c.rs"]);
    assert!(t.errors.is_empty(), "{:?}", t.errors);
    assert_eq!(t.dialed.len(), 4, "two reads, the refused dial, the retry");
    let len = |i: usize| t.dialed[i].chars().count();
    assert!(len(2) > N, "{}", len(2));
    assert!(len(3) < N, "the retry is {} chars", len(3));
    let retry = &t.dialed[3];
    assert!(
        !retry.contains("From earlier work"),
        "recall kept:\n{retry}"
    );
    assert!(retry.starts_with(
        "PROJECT CARD: crew, a GPU terminal\n\nPROJECT INSTRUCTIONS: keep files short\n\n\
         TASK:\nwhat is in a.rs, b.rs and c.rs?"
    ));
    assert!(retry.contains(
        "CALLED sys:read_file {\"path\":\"a.rs\"}\nCALLED sys:read_file {\"path\":\"b.rs\"}\n"
    ));
    assert!(!retry.contains("a.rs line 000") && !retry.contains("b.rs line 000"));
    assert!(retry.contains(&file("c.rs")), "the last round is whole");
    assert_eq!(t.notes, 1, "said once");
}

/// After the cut only the round being made is shown whole, so `c.rs`, whole
/// in the retry, is a call line by the time a repeat's answer is read: asked
/// for again, it runs rather than points at round 3.
#[test]
fn a_read_asked_for_again_after_the_cut_runs_again() {
    let t = turn(&["b.rs", "c.rs", "c.rs"], DASHSCOPE, None);
    assert_eq!(t.ran, ["a.rs", "b.rs", "c.rs", "c.rs"]);
    let last = t.dialed.last().unwrap();
    assert!(!last.contains("same call as round"), "{last}");
    assert!(last.contains(&file("c.rs")));
}

#[test]
fn an_overflow_on_every_dial_is_two_tries_then_the_error() {
    let t = turn(&["b.rs", "c.rs"], DASHSCOPE, Some(2));
    assert_eq!(t.dialed.len(), 4, "two reads and exactly two tries");
    assert_eq!(t.errors.len(), 1, "{:?}", t.errors);
    assert!(
        t.errors[0].contains("Range of input length"),
        "{:?}",
        t.errors
    );
    assert_eq!(t.notes, 1);
}

#[test]
fn another_400_is_not_retried() {
    let t = turn(&["b.rs", "c.rs"], BAD_INPUT, None);
    assert_eq!(t.dialed.len(), 3, "no second try");
    assert_eq!(t.errors.len(), 1, "{:?}", t.errors);
    assert!(t.errors[0].contains("unsupported input format"));
    assert_eq!(t.notes, 0);
}
