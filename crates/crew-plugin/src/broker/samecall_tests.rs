//! A read the turn already made, with nothing written since, is not run again
//! (`crew_hive::tools::seen`). Each test counts REAL executions on a runner
//! that classifies tools the way the session does (`tier_of`).
use std::sync::{Arc, Mutex};

use super::*;
use crate::broker::tier::{tier_of, Tier};
use crate::broker::turnkit::{turn as relay, Script};

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
/// `then`, then `@done`; hand back the calls that ran, the prompts dialed and
/// the tool lane's events, read as cards.
fn turn(
    result: Result<String, String>,
    first: &str,
    then: &[&str],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let runner = Arc::new(Counting {
        ran: Mutex::new(Vec::new()),
        result,
    });
    let tools: Arc<dyn ToolRunner> = runner.clone();
    let mut replies = vec![first];
    replies.extend(then);
    replies.push("@done");
    let t = relay(Script::new(&replies), |b| b.with_tools(tools), "task");
    let ran = runner.ran.lock().unwrap().clone();
    (ran, t.prompts.clone(), t.cards())
}

const READ: &str = "reading\n@tool sys:read_file {\"path\":\"a.rs\"}";

#[test]
fn the_same_read_twice_runs_once_and_the_second_points_at_the_first() {
    let (ran, dialed, cards) = turn(
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
    // The pane's tool line says so too: the second read's result points at
    // the first.
    let second = cards
        .iter()
        .filter(|c| c.starts_with("[tool] sys:read_file \u{2713}"))
        .nth(1)
        .expect("a second result");
    assert!(second.contains("same call as round 1"), "{second}");
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
