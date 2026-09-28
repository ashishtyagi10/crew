//! A read the turn already made, with nothing written since, is not run again
//! (`crew_hive::tools::seen`). Each test counts REAL executions on a runner
//! that classifies tools the way the session does (`tier_of`).
use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::broker::tier::{tier_of, Tier};
use crate::Registry;

/// Replies in order, then `@done`; keeps every prompt it was dialed with.
struct Scripted(Mutex<Vec<String>>, Mutex<Vec<String>>);

impl Scripted {
    fn new(replies: &[&str]) -> Self {
        let v = replies.iter().rev().map(|s| s.to_string()).collect();
        Self(Mutex::new(v), Mutex::new(Vec::new()))
    }
}

impl Adapter for Scripted {
    fn name(&self) -> &str {
        "planner"
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

/// Counts every call it actually runs. `read_file` answers `result`.
struct Counting {
    ran: Mutex<Vec<String>>,
    result: Result<String, String>,
}

impl ToolRunner for Counting {
    fn hint(&self) -> String {
        "TOOLS: sys:read_file sys:edit".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.ran
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        match tool {
            "read_file" => self.result.clone(),
            _ => Ok("edited".into()),
        }
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        tier_of(server, tool) == Tier::Read
    }
}

/// Run a turn whose first reply is `first` and whose later replies are
/// `then`; hand back the calls that ran, the prompts dialed and the hops.
fn turn(
    result: Result<String, String>,
    first: &str,
    then: &[&str],
) -> (Vec<String>, Vec<String>, Vec<Hop>) {
    let runner = std::sync::Arc::new(Counting {
        ran: Mutex::new(Vec::new()),
        result,
    });
    let b =
        Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5)).with_tools(runner.clone());
    let agent = Scripted::new(then);
    let mut hops = Vec::new();
    b.run_tools(
        &agent,
        "base prompt",
        first.to_string(),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "planner", "t1", "task"),
        &HopStream::noop(),
        &mut |h| hops.push(h),
    );
    let ran = runner.ran.lock().unwrap().clone();
    let dialed = agent.1.lock().unwrap().clone();
    (ran, dialed, hops)
}

const READ: &str = "reading\n@tool sys:read_file {\"path\":\"a.rs\"}";

#[test]
fn the_same_read_twice_runs_once_and_the_second_points_at_the_first() {
    let (ran, dialed, hops) = turn(
        Ok("FILE BODY".into()),
        READ,
        &[
            "again\n@tool sys:read_file {\"path\":\"a.rs\"}",
            "answer\n@done",
        ],
    );
    assert_eq!(ran.len(), 1, "the repeat must not run: {ran:?}");
    // The second follow-up carries both exchanges: the first with the body,
    // the second with the pointer to it, and the body only once.
    let last = dialed.last().expect("dialed twice");
    assert!(
        last.contains("same call as round 1"),
        "no pointer in:\n{last}"
    );
    assert_eq!(last.matches("FILE BODY").count(), 1, "{last}");
    // The card says so too, on its outcome line.
    let card = hops
        .iter()
        .filter(|h| h.from == "sys:read_file" && h.text.starts_with("[tool]"))
        .nth(1)
        .expect("a second result card");
    let head = card.text.lines().next().unwrap();
    assert!(
        head.contains("\u{2713}") && head.ends_with("same as round 1"),
        "{head}"
    );
}

#[test]
fn spacing_and_key_order_are_the_same_call() {
    let (ran, dialed, _) = turn(
        Ok("FILE BODY".into()),
        "reading\n@tool sys:read_file {\"path\": \"a.rs\", \"offset\": 0}",
        &["again\n@tool sys:read_file {\"offset\":0,\"path\":\"a.rs\"}"],
    );
    assert_eq!(ran.len(), 1, "{ran:?}");
    assert!(dialed.last().unwrap().contains("same call as round 1"));
}

#[test]
fn an_edit_between_two_reads_makes_the_second_run() {
    let (ran, dialed, _) = turn(
        Ok("FILE BODY".into()),
        READ,
        &[
            "fixing\n@tool sys:edit {\"path\":\"a.rs\",\"old\":\"x\",\"new\":\"y\"}",
            "checking\n@tool sys:read_file {\"path\":\"a.rs\"}",
        ],
    );
    let reads = ran.iter().filter(|c| c.contains("read_file")).count();
    assert_eq!(reads, 2, "the file may have changed: {ran:?}");
    assert!(!dialed.last().unwrap().contains("same call as"));
}

#[test]
fn a_failed_read_is_tried_again() {
    let (ran, dialed, _) = turn(
        Err("no such file".into()),
        READ,
        &["again\n@tool sys:read_file {\"path\":\"a.rs\"}"],
    );
    assert_eq!(ran.len(), 2, "{ran:?}");
    assert!(!dialed.last().unwrap().contains("same call as"));
}

/// The real session surface classifies by the approval gate's tier table:
/// reads repeat, writes and the shell do not, nor does a server nobody knows,
/// nor diagnostics, which arrive on the language server's clock.
#[test]
fn the_session_says_which_calls_only_look() {
    let session = crate::broker::session::Session::default();
    let tools = session.tools_with_sys(true).expect("sys tools");
    for (server, tool) in [("sys", "read_file"), ("sys", "grep"), ("lsp", "hover")] {
        assert!(tools.repeatable(server, tool), "{server}:{tool}");
    }
    for (server, tool) in [
        ("sys", "edit"),
        ("sys", "write_file"),
        ("sys", "run"),
        ("mail", "send"),
        ("lsp", "diagnostics"),
    ] {
        assert!(!tools.repeatable(server, tool), "{server}:{tool}");
    }
}
