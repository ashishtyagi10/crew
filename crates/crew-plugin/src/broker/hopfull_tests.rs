//! A relay turn whose follow-up outgrows the model's context: cut once and
//! dialed again, and only for that error. The cut is the harness's — every
//! older result down to its call line (`Exchanges::tighten`) — and the
//! relay's: the recalled turns leave the frame (`TurnSpec::shed`). Each test
//! runs the turn against a scripted agent with a window and reads what it was
//! really sent.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::broker::adapter::Adapter;
use crate::broker::hop::HopKind;
use crate::broker::tier::{tier_of, Tier};
use crate::broker::toolcall::ToolRunner;
use crate::broker::Broker;
use crate::{PluginEvent, Registry};
use crew_hive::provider::ProviderError;

const DASHSCOPE: &str = r#"{"error":{"message":"Range of input length should be [1, 30720]","code":"invalid_parameter_error"}}"#;
const BAD_INPUT: &str = r#"{"error":{"message":"<400> InternalError.Algo.InvalidParameter: An unknown error occurred due to an unsupported input format.","code":"invalid_parameter_error"}}"#;

/// The window, in chars: the frame and two reads fit, the third's follow-up
/// (the first read shortened, `b.rs` and `c.rs` whole) does not.
const N: usize = 9_000;

const RECALLED: &str = "From earlier work in this project:\n- 3d ago \u{2014} you asked: \
                        why is the router slow\n- files that came up: src/route.rs";

/// The message as the relay builds it: a card, a recalled block, the ask.
fn message() -> String {
    format!("PROJECT CARD: crew, a GPU terminal\n\n{RECALLED}\n\nwhat is in a.rs, b.rs and c.rs?")
}

/// Refuses, as an API agent words it, a dial over [`N`] chars and every dial
/// from the `from`-th (from 0); otherwise the next reply. Keeps every dial.
struct Window {
    replies: Mutex<Vec<String>>,
    dialed: Arc<Mutex<Vec<String>>>,
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
            false => Ok(self.replies.lock().unwrap().remove(0)),
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
    answer: Option<String>,
    ran: Vec<String>,
    dialed: Vec<String>,
    errors: Vec<String>,
}

/// A turn that reads `a.rs`, then each of `then`, then answers.
fn turn(then: &[&str], refusal: &'static str, from: Option<usize>) -> Turn {
    let runner = Arc::new(Files(Mutex::new(Vec::new())));
    let mut replies: Vec<String> = std::iter::once("a.rs")
        .chain(then.iter().copied())
        .map(read)
        .collect();
    replies.push("all read\n@done".into());
    let dialed: Arc<Mutex<Vec<String>>> = Arc::default();
    let agent = Window {
        replies: Mutex::new(replies),
        dialed: Arc::clone(&dialed),
        refusal,
        from,
    };
    let b = Broker::new(
        Registry::new(vec![Box::new(agent)]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(runner.clone())
    .with_tool_rounds(8);
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    b.run("user", "smith", &message(), "t1", &emit, &mut |h| {
        hops.push(h)
    });
    let of = |kind| {
        hops.iter()
            .filter(move |h| h.kind == kind)
            .map(|h| h.text.clone())
    };
    let answer = of(HopKind::Done).next();
    let errors = of(HopKind::Error).collect();
    let ran = runner.0.lock().unwrap().clone();
    let dialed = dialed.lock().unwrap().clone();
    Turn {
        answer,
        ran,
        dialed,
        errors,
    }
}

#[test]
fn the_third_follow_up_overflows_is_cut_and_the_turn_answers() {
    let t = turn(&["b.rs", "c.rs"], DASHSCOPE, None);
    assert_eq!(t.answer.as_deref(), Some("all read"));
    assert_eq!(t.ran, ["a.rs", "b.rs", "c.rs"]);
    assert!(t.errors.is_empty(), "{:?}", t.errors);
    assert_eq!(
        t.dialed.len(),
        5,
        "the frame, two reads, the refused dial, the retry"
    );
    let len = |i: usize| t.dialed[i].chars().count();
    assert!(len(3) > N, "{}", len(3));
    assert!(len(4) < N, "the retry is {} chars", len(4));
    let retry = &t.dialed[4];
    assert!(
        !retry.contains("From earlier work"),
        "recall kept:\n{retry}"
    );
    assert!(
        retry.contains("PROJECT CARD: crew, a GPU terminal\n\nwhat is in a.rs"),
        "{retry}"
    );
    assert!(retry.contains(
        "CALLED sys:read_file {\"path\":\"a.rs\"}\nCALLED sys:read_file {\"path\":\"b.rs\"}\n"
    ));
    assert!(!retry.contains("a.rs line 000") && !retry.contains("b.rs line 000"));
    assert!(retry.contains(&file("c.rs")), "the last round is whole");
}

#[test]
fn an_overflow_on_every_dial_is_two_tries_then_the_error() {
    let t = turn(&["b.rs", "c.rs"], DASHSCOPE, Some(3));
    assert_eq!(
        t.dialed.len(),
        5,
        "the frame, two reads and exactly two tries"
    );
    assert_eq!(t.errors.len(), 1, "{:?}", t.errors);
    assert!(
        t.errors[0].contains("Range of input length"),
        "{:?}",
        t.errors
    );
}

#[test]
fn another_400_is_not_retried() {
    let t = turn(&["b.rs", "c.rs"], BAD_INPUT, None);
    assert_eq!(t.dialed.len(), 4, "no second try");
    assert_eq!(t.errors.len(), 1, "{:?}", t.errors);
    assert!(t.errors[0].contains("unsupported input format"));
}
