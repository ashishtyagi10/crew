use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::text::HEAD;
use super::*;

/// The briefs an advisor was given, in order.
pub(super) type Briefs = Arc<Mutex<Vec<String>>>;

/// What one fake advice call cost.
const SPENT: Spent = Spent {
    input: 40,
    output: 9,
    micros_usd: 120,
};

/// The failure of a build that did not build.
pub(super) const BROKEN: &str = "exit 101\n   Compiling crew v0.1.0\nerror[E0308]: \
                                 mismatched types\n  --> src/a.rs:3:5\nerror: could not compile";

type Reply = Result<&'static str, &'static str>;

thread_local! {
    /// The advisor this thread's test installed.
    static INSTALLED: RefCell<Option<Arc<Ask>>> = const { RefCell::new(None) };
}

/// An advisor that answers `reply` and keeps every brief it was given.
fn fake(reply: Reply, briefs: Briefs) -> Box<Ask> {
    Box::new(move |brief: &str| {
        briefs.lock().unwrap().push(brief.to_string());
        reply
            .map(|r| (r.to_string(), SPENT))
            .map_err(str::to_string)
    })
}

/// The seam `live` reads under test: this thread's advisor, else none.
pub(super) fn installed() -> Option<Box<Ask>> {
    let ask = INSTALLED.with(|i| i.borrow().clone())?;
    Some(Box::new(move |brief: &str| ask(brief)))
}

/// Advice as this thread's turns get it, until the guard drops.
pub(super) struct Installed;

impl Drop for Installed {
    fn drop(&mut self) {
        INSTALLED.with(|i| *i.borrow_mut() = None);
    }
}

/// `ask` as this thread's advisor.
pub(super) fn install(ask: Box<Ask>) -> Installed {
    INSTALLED.with(|i| *i.borrow_mut() = Some(Arc::from(ask)));
    Installed
}

pub(super) fn advising(reply: Reply) -> (Briefs, Installed) {
    let briefs = Briefs::default();
    let on = install(fake(reply, Arc::clone(&briefs)));
    (briefs, on)
}

/// A surface whose `sys:run` fails on exit 101, as `SessionTools` says.
pub(super) struct Runs;

impl ToolRunner for Runs {
    fn hint(&self) -> String {
        "TOOLS: sys:run".into()
    }
    fn call(&self, _s: &str, _t: &str, args: &str) -> Result<String, String> {
        Ok(match args.contains("build") {
            true => BROKEN.into(),
            false => "exit 0\nfine".into(),
        })
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        (s, t) == ("sys", "run") && out.starts_with("exit 101")
    }
}

fn advisor(reply: Reply) -> (Advisor, Briefs) {
    let briefs = Briefs::default();
    let ask = fake(reply, Arc::clone(&briefs));
    let advisor = Advisor {
        ask: Some(Some(ask)),
        ..Default::default()
    };
    (advisor, briefs)
}

fn run(cmd: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "run".into(),
        args: format!("{{\"cmd\": \"{cmd}\"}}"),
    }
}

/// One round of `call`, run on [`Runs`], after a stop when `stopped`.
fn round_of(a: &mut Advisor, call: ToolCall, stats: &mut RunStats, stopped: bool) -> String {
    let text = Runs.call(&call.server, &call.tool, &call.args).unwrap();
    let ran = Ran {
        ok: !Runs.failed(&call.server, &call.tool, &text),
        text,
        ms: 0,
    };
    let env = Envelope::new("user", "engineer", "t1", "make it build");
    a.read(&Runs, &env, &[call], &[ran], stats, stopped)
}

fn round(a: &mut Advisor, call: ToolCall, stats: &mut RunStats) -> String {
    round_of(a, call, stats, false)
}

#[test]
fn a_failed_run_is_read_once_and_its_advice_heads_a_section() {
    let (mut a, briefs) = advisor(Ok("- E0308 at src/a.rs:3: the caller wants &str\n\n- fix \
                                     the call, then rerun cargo build\n- third\n- fourth"));
    let mut stats = RunStats::default();
    let added = round(&mut a, run("cargo build"), &mut stats);
    assert_eq!(
        added,
        format!(
            "{HEAD}\n- E0308 at src/a.rs:3: the caller wants &str\n- fix the call, then \
             rerun cargo build\n- third\n\n"
        )
    );
    let briefs = briefs.lock().unwrap();
    assert_eq!(briefs.len(), 1);
    let b = &briefs[0];
    assert!(b.contains("THE REQUEST:\nmake it build"), "{b}");
    assert!(
        b.contains("1. sys:run {\"cmd\": \"cargo build\"} \u{2192} failed"),
        "{b}"
    );
    assert!(b.contains("THE COMMAND THAT FAILED: cargo build"), "{b}");
    assert!(
        b.contains("error[E0308]") && !b.contains("Compiling"),
        "cut: {b}"
    );
    assert_eq!(
        (stats.tok_in, stats.tok_out, stats.cost_microusd),
        (40, 9, 120)
    );
}

#[test]
fn a_round_with_no_failure_asks_nothing_and_adds_nothing() {
    let (mut a, briefs) = advisor(Ok("unused"));
    let added = round(&mut a, run("cargo test"), &mut RunStats::default());
    assert_eq!(added, "");
    assert!(briefs.lock().unwrap().is_empty());
}

#[test]
fn a_turn_asks_at_most_twice_and_keeps_the_newest_advice() {
    let (mut a, briefs) = advisor(Ok("rerun with the fix"));
    let mut stats = RunStats::default();
    for _ in 0..3 {
        let added = round(&mut a, run("cargo build"), &mut stats);
        assert_eq!(added, format!("{HEAD}\n- rerun with the fix\n\n"));
    }
    assert_eq!(briefs.lock().unwrap().len(), PER_TURN as usize);
    let second = &briefs.lock().unwrap()[1];
    assert!(second.contains("2. sys:run"), "the calls so far: {second}");
}

#[test]
fn an_advisor_that_errors_adds_nothing() {
    let (mut a, briefs) = advisor(Err("timed out after 30s"));
    let mut stats = RunStats::default();
    assert_eq!(round(&mut a, run("cargo build"), &mut stats), "");
    assert_eq!(briefs.lock().unwrap().len(), 1);
    assert_eq!(stats, RunStats::default(), "nothing billed");
    let (mut a, _) = advisor(Ok("\n```\n"));
    assert_eq!(
        round(&mut a, run("cargo build"), &mut stats),
        "",
        "said nothing"
    );
}

/// A stop pressed while the tools ran ends the turn before any follow-up:
/// a read then would be paid for and never sent.
#[test]
fn a_stopped_turn_is_not_read() {
    let (mut a, briefs) = advisor(Ok("unused"));
    let mut stats = RunStats::default();
    assert_eq!(round_of(&mut a, run("cargo build"), &mut stats, true), "");
    assert!(briefs.lock().unwrap().is_empty());
    assert_eq!(stats, RunStats::default(), "nothing billed");
}
