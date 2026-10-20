//! One relay reply's tool calls, run in one round.
//!
//! An agent that needed three files read them in three rounds, three model
//! calls each re-sending the whole prompt, although it knew all three paths
//! from the start. It may now write them on its reply's last lines
//! (`split_tool_calls`), and they are answered together in one follow-up.
//! Reads also RUN together, a thread each: `SessionTools` is `Sync`, with every
//! host behind its own lock, and a read changes nothing another read could
//! see. A batch with anything else in it runs in the order written, because an
//! edit and the read after it mean something only in that order.
use std::thread::ScopedJoinHandle;
use std::time::Instant;

use super::toolcall::ToolRunner;
use super::toolround::{run_once, settle};
use crew_hive::tools::exchanges::Exchanges;
use crew_hive::tools::{seen::Seen, ToolCall};

/// Most calls one reply may make. Past it a call is answered "not run", so
/// a model that wrote a list of twenty reads learns why it got four.
pub(super) const PER_REPLY: usize = 4;

/// One call of a batch, run or answered: what its card and exchange need.
pub(super) struct Ran {
    pub(super) ok: bool,
    pub(super) text: String,
    pub(super) ms: u64,
}

impl Ran {
    /// A call answered without running, and why.
    pub(super) fn not_run(why: &str) -> Self {
        Self {
            ok: false,
            text: why.to_string(),
            ms: 0,
        }
    }
}

/// A call's output, and how long it took in milliseconds.
type Called = (Result<String, String>, u64);

/// A read of a batch between its start and its result.
enum Job<'s> {
    /// A repeat, answered by pointing at the round that made it first.
    Pointer(u32),
    /// Running on a thread of its own.
    Running(ScopedJoinHandle<'s, Called>),
}

/// Run the calls one reply made and answer each, in the order written. `log`
/// is the turn's exchanges before this round; every call is noted in `seen`
/// under the entry its exchange will have, so a repeat finds the one call it
/// repeats.
pub(super) fn run(
    runner: &dyn ToolRunner,
    seen: &mut Seen,
    log: &Exchanges,
    calls: &[ToolCall],
) -> Vec<Ran> {
    let entry = |k: usize| log.next_entry() + k as u32;
    let reads = calls.iter().all(|c| runner.repeatable(&c.server, &c.tool));
    if calls.len() < 2 || !reads {
        let one = |(k, call): (usize, &ToolCall)| {
            let started = Instant::now();
            let (ok, text) = run_once(runner, seen, log, call, entry(k));
            let ms = started.elapsed().as_millis() as u64;
            Ran { ok, text, ms }
        };
        return calls.iter().enumerate().map(one).collect();
    }
    // The reads this batch has started: a second copy of one in the same
    // reply points at the first rather than running beside it.
    let mut planned = seen.clone();
    std::thread::scope(|s| {
        let mut jobs = Vec::with_capacity(calls.len());
        for (k, call) in calls.iter().enumerate() {
            jobs.push(match log.repeat(&planned, call) {
                Some(round) => Job::Pointer(round),
                None => {
                    planned.ran(runner, call, entry(k), true);
                    Job::Running(s.spawn(move || timed(runner, call)))
                }
            });
        }
        let mut out = Vec::with_capacity(calls.len());
        for (k, (job, call)) in jobs.into_iter().zip(calls).enumerate() {
            out.push(match job {
                Job::Pointer(round) => Ran {
                    ok: true,
                    text: Seen::pointer(round),
                    ms: 0,
                },
                Job::Running(h) => {
                    let (called, ms) = h
                        .join()
                        .unwrap_or_else(|_| (Err("the tool's thread panicked".into()), 0));
                    let (ok, text) = settle(runner, call, called);
                    seen.ran(runner, call, entry(k), ok);
                    Ran { ok, text, ms }
                }
            });
        }
        out
    })
}

/// `call`, run and timed where it runs.
fn timed(runner: &dyn ToolRunner, call: &ToolCall) -> Called {
    let started = Instant::now();
    let called = runner.call(&call.server, &call.tool, &call.args);
    (called, started.elapsed().as_millis() as u64)
}

/// What a call past [`PER_REPLY`] is answered with. The turn's budget is
/// rounds, so a reply that is run at all runs this far.
pub(super) fn refusal() -> String {
    format!("not run \u{2014} at most {PER_REPLY} tool calls per reply")
}

#[cfg(test)]
#[path = "toolbatch_tests.rs"]
mod tests;
