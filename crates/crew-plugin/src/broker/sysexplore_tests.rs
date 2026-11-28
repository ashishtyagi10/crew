//! `sys:explore` against a scripted model: what the nested agent may call,
//! what the caller is handed back, and how it ends.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crew_hive::{Completion, CompletionRequest, Provider, ProviderError, ToolInvocation};

use super::*;
use crate::broker::sysexplorekit::{Reader, ALLOWED};

type Reply = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// Replays its replies, keeps every request, and may hang instead.
pub(super) struct Scripted {
    replies: Mutex<Vec<Completion>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
    pub(super) hang: bool,
}

pub(super) fn script(
    replies: Vec<Completion>,
) -> (Arc<Scripted>, Arc<Mutex<Vec<CompletionRequest>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let p = Scripted {
        replies: Mutex::new(replies.into_iter().rev().collect()),
        seen: Arc::clone(&seen),
        hang: false,
    };
    (Arc::new(p), seen)
}

impl Provider for Scripted {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(&self, req: CompletionRequest) -> Reply {
        self.seen.lock().unwrap().push(req);
        let hang = self.hang;
        let c = self.replies.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move {
            if hang {
                std::future::pending::<()>().await;
            }
            Ok(c)
        })
    }
}

pub(super) fn calling(name: &str, input: serde_json::Value) -> Completion {
    Completion {
        input_tokens: 100,
        output_tokens: 10,
        calls: vec![ToolInvocation {
            id: "t1".into(),
            name: name.into(),
            input,
            bad_args: None,
        }],
        ..Default::default()
    }
}

pub(super) fn answering(text: &str) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 100,
        output_tokens: 10,
        ..Default::default()
    }
}

/// A file under a fresh temp dir holding `body`; its path.
pub(super) fn file(tag: &str, body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("crew-explore-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.txt");
    std::fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

pub(super) fn go(
    p: Arc<Scripted>,
    limits: Limits,
    stop: bool,
    bill: &Bill,
) -> Result<String, String> {
    let stop = Arc::new(AtomicBool::new(stop));
    explore(
        r#"{"goal":"where is x"}"#,
        p,
        "m",
        limits,
        &stop,
        bill,
        None,
    )
}

#[test]
fn the_caller_gets_the_summary_and_a_count_not_the_files() {
    let path = file("summary", "SECRET-FILE-BODY\n");
    let (p, _) = script(vec![
        calling("sys__read_file", serde_json::json!({"path": path})),
        answering("x lives at a.txt:1"),
    ]);
    let out = go(p, Limits::default(), false, &Bill::default()).unwrap();
    assert_eq!(
        out,
        format!("{UNTRUSTED}\nx lives at a.txt:1\nexplored: 1 files read, 0 searches")
    );
}

#[test]
fn only_read_tools_are_offered_and_only_they_run() {
    let path = file("ro", "keep");
    let target = format!("{path}.new");
    let (p, seen) = script(vec![
        calling(
            "sys__write_file",
            serde_json::json!({"path": target, "content": "x"}),
        ),
        answering("done"),
    ]);
    go(p, Limits::default(), false, &Bill::default()).unwrap();
    assert!(!std::path::Path::new(&target).exists(), "a write ran");
    let offered: Vec<String> = seen.lock().unwrap()[0]
        .tools
        .iter()
        .map(|t| t.name.clone())
        .collect();
    for want in ALLOWED {
        assert!(
            offered.contains(&format!("sys__{want}")),
            "{want} missing: {offered:?}"
        );
    }
    assert_eq!(offered.len(), ALLOWED.len(), "{offered:?}");
    let r = Reader::default();
    for bad in [
        "write_file",
        "edit",
        "run",
        "fetch",
        "search",
        "explore",
        "todo",
    ] {
        assert!(r.call("sys", bad, "{}").is_err(), "{bad} ran");
    }
    assert!(r.call("mcp", "read_file", "{}").is_err());
}

#[test]
fn it_cannot_call_itself() {
    let (p, seen) = script(vec![
        calling("sys__explore", serde_json::json!({"goal": "again"})),
        answering("fine"),
    ]);
    let out = go(p, Limits::default(), false, &Bill::default()).unwrap();
    assert!(out.contains("\nfine\n"), "{out}");
    assert!(seen.lock().unwrap()[0]
        .tools
        .iter()
        .all(|t| !t.name.contains("explore")));
}

#[test]
fn a_spent_budget_still_returns_what_it_found() {
    // Distinct files: a repeat of one read is answered by pointer, not run.
    let call = |tag: &str| {
        calling(
            "sys__read_file",
            serde_json::json!({"path": file(tag, "x")}),
        )
    };
    let (p, seen) = script(vec![
        call("b1"),
        call("b2"),
        answering("found it at a.txt:1"),
    ]);
    let limits = Limits {
        rounds: 1,
        ..Limits::default()
    };
    let out = go(p, limits, false, &Bill::default()).unwrap();
    assert!(out.contains("\nfound it at a.txt:1"), "{out}");
    // The spent pool ends with a last word asked WITHOUT tools.
    assert!(seen.lock().unwrap().last().unwrap().tools.is_empty());
}
