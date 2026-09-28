//! Several calls on a relay reply's last lines are one round: one follow-up
//! answers them all, reads run together, and every call counts against the
//! turn's budget. Each test runs a whole turn against a scripted agent.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::broker::adapter::{Adapter, HopStream, Usage};
use crate::broker::hop::{Hop, HopKind, RunStats};
use crate::broker::tier::{tier_of, Tier};
use crate::broker::toolcall::ToolRunner;
use crate::broker::{Broker, Envelope};
use crate::Registry;

/// Replies in order, then `@done`; keeps every prompt it was dialed with.
struct Scripted(Mutex<Vec<String>>, Mutex<Vec<String>>);

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

/// Records each call as it starts, sleeps `nap`, answers `<tool> of <args>`.
struct Files {
    ran: Mutex<Vec<String>>,
    nap: Duration,
}

impl ToolRunner for Files {
    fn hint(&self) -> String {
        "TOOLS: sys:read_file sys:edit".into()
    }
    fn call(&self, _server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.ran.lock().unwrap().push(format!("{tool} {args}"));
        std::thread::sleep(self.nap);
        Ok(format!("{tool} of {args}"))
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        tier_of(server, tool) == Tier::Read
    }
}

/// What a turn did: the calls that ran, the prompts dialed after the first
/// reply, the hops, the answer, and how long the whole turn took.
struct Turn {
    ran: Vec<String>,
    dialed: Vec<String>,
    hops: Vec<Hop>,
    answer: String,
    took: Duration,
}

fn turn(first: &str, then: &[&str], calls: u32, nap: Duration) -> Turn {
    let runner = Arc::new(Files {
        ran: Mutex::new(Vec::new()),
        nap,
    });
    let b = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(runner.clone())
        .with_tool_rounds(calls);
    let agent = Scripted(
        Mutex::new(then.iter().rev().map(|s| s.to_string()).collect()),
        Mutex::new(Vec::new()),
    );
    let mut hops = Vec::new();
    let started = Instant::now();
    let answer = b.run_tools(
        &agent,
        "base prompt",
        first.to_string(),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "planner", "t1", "task"),
        &HopStream::noop(),
        &mut |h| hops.push(h),
    );
    Turn {
        took: started.elapsed(),
        ran: runner.ran.lock().map(|r| r.clone()).unwrap(),
        dialed: agent.1.into_inner().unwrap(),
        hops,
        answer,
    }
}

/// A turn with calls that take no time, under the solo budget of eight.
fn quick(first: &str, then: &[&str]) -> Turn {
    turn(first, then, 8, Duration::ZERO)
}

fn reads(said: &str, paths: &[&str]) -> String {
    let calls: Vec<String> = paths
        .iter()
        .map(|p| format!("@tool sys:read_file {{\"path\":\"{p}\"}}"))
        .collect();
    format!("{said}\n{}", calls.join("\n"))
}

#[test]
fn three_reads_on_the_last_lines_are_answered_in_one_follow_up() {
    let first = reads("Reading the three route files.", &["a.rs", "b.rs", "c.rs"]);
    let t = quick(&first, &["answer\n@done"]);
    assert_eq!(t.ran.len(), 3, "{:?}", t.ran);
    assert_eq!(t.dialed.len(), 1, "one follow-up for the three");
    assert_eq!(t.answer, "answer\n@done");
    let f = &t.dialed[0];
    for p in ["a.rs", "b.rs", "c.rs"] {
        let result = format!("RESULT:\nread_file of {{\"path\":\"{p}\"}}");
        assert!(f.contains(&result), "{p} missing from:\n{f}");
    }
    assert_eq!(f.matches("CALLED sys:read_file").count(), 3, "{f}");
    assert_eq!(f.matches("Reading the three").count(), 1, "said once:\n{f}");
    assert!(f.contains("You may make 5 more tool call(s)"), "{f}");
}

#[test]
fn a_batch_of_reads_runs_at_once() {
    let nap = Duration::from_millis(250);
    let t = turn(&reads("", &["a", "b", "c"]), &["ok\n@done"], 8, nap);
    assert_eq!(t.ran.len(), 3);
    assert!(t.took < nap * 2, "took {:?}", t.took);
}

#[test]
fn a_batch_with_an_edit_runs_in_the_order_written() {
    let first = "fixing\n@tool sys:read_file {\"path\":\"a\"}\n\
                 @tool sys:edit {\"path\":\"a\"}\n@tool sys:read_file {\"path\":\"a\"}";
    let nap = Duration::from_millis(100);
    let t = turn(first, &["ok\n@done"], 8, nap);
    let order: Vec<&str> = t.ran.iter().map(|r| &r[..r.find(' ').unwrap()]).collect();
    assert_eq!(order, ["read_file", "edit", "read_file"]);
    assert!(t.took >= nap * 3, "took {:?}", t.took);
}

/// Budget 8, two replies of four reads: eight calls run, and the third ask
/// meets the budget-spent path, its text kept and its calls cut.
#[test]
fn each_call_of_a_batch_counts_against_the_budget() {
    let then = [
        reads("next four", &["e", "f", "g", "h"]),
        reads("one more", &["i"]),
    ];
    let t = quick(
        &reads("first four", &["a", "b", "c", "d"]),
        &[&then[0], &then[1]],
    );
    assert_eq!(t.ran.len(), 8, "{:?}", t.ran);
    assert_eq!(t.dialed.len(), 2);
    assert!(t.dialed[1].contains("This was your LAST tool call"));
    assert_eq!(t.answer, "one more");
    let spent = t
        .hops
        .iter()
        .find(|h| h.kind == HopKind::Terminated)
        .unwrap();
    assert!(
        spent.text.contains("tool budget spent \u{2014} 8 calls"),
        "{spent:?}"
    );
}

/// Past four in one reply, or past what the budget has left, a call is
/// answered "not run" in the log rather than dropped without a word.
#[test]
fn calls_past_the_cap_or_the_budget_are_answered_not_run() {
    let t = quick(&reads("", &["a", "b", "c", "d", "e", "f"]), &["ok\n@done"]);
    assert_eq!(t.ran.len(), 4);
    let cap = t.dialed[0].matches("at most 4 tool calls per reply");
    assert_eq!(cap.count(), 2, "{}", t.dialed[0]);
    let t = turn(
        &reads("", &["a", "b", "c"]),
        &["ok\n@done"],
        2,
        Duration::ZERO,
    );
    assert_eq!(t.ran.len(), 2);
    assert!(t.dialed[0].contains("tool budget spent (2 calls this turn)"));
}

#[test]
fn a_read_written_twice_in_one_batch_runs_once() {
    let t = quick(&reads("", &["a", "b", "a"]), &["ok\n@done"]);
    assert_eq!(t.ran.len(), 2, "{:?}", t.ran);
    let f = &t.dialed[0];
    assert!(f.contains("same call as round 1"), "{f}");
}
