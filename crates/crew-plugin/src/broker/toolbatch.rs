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

use super::hop::{Hop, HopKind};
use super::toolcall::{ToolRunner, RESULT_CLIP};
use super::toolclip::clip_result;
use super::toolline::{call_line, result_line, same_as};
use super::toolround::{run_once, settle};
use super::Envelope;
use crew_hive::tools::exchanges::Exchanges;
use crew_hive::tools::{seen::Seen, ToolCall};

/// Most calls one reply may make. Past it a call is answered "not run", so
/// a model that wrote a list of twenty reads learns why it got four.
pub(super) const PER_REPLY: usize = 4;

/// One call of a batch, run or answered: what its card and exchange need.
pub(super) struct Ran {
    pub(super) ok: bool,
    pub(super) text: String,
    /// The round whose result stands for this one, when it was not run.
    pub(super) repeat: Option<u32>,
    pub(super) ms: u64,
}

impl Ran {
    /// A call answered without running, and why.
    pub(super) fn not_run(why: &str) -> Self {
        Self {
            ok: false,
            text: why.to_string(),
            repeat: None,
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
            let (ok, text, repeat) = run_once(runner, seen, log, call, entry(k));
            let ms = started.elapsed().as_millis() as u64;
            Ran {
                ok,
                text,
                repeat,
                ms,
            }
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
                    repeat: Some(round),
                    ms: 0,
                },
                Job::Running(h) => {
                    let (called, ms) = h
                        .join()
                        .unwrap_or_else(|_| (Err("the tool's thread panicked".into()), 0));
                    let (ok, text) = settle(runner, call, called);
                    seen.ran(runner, call, entry(k), ok);
                    Ran {
                        ok,
                        text,
                        repeat: None,
                        ms,
                    }
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

/// What a call that is not run is answered with: past [`PER_REPLY`], or past
/// what is left of the turn's `max` calls when that is what stopped it.
pub(super) fn refusal(budget_bound: bool, max: u32) -> String {
    match budget_bound {
        true => format!("not run \u{2014} tool budget spent ({max} calls this turn)"),
        false => format!("not run \u{2014} at most {PER_REPLY} tool calls per reply"),
    }
}

/// The card for a call as it is made: `[tool] sys:read_file  src/lib.rs`.
pub(super) fn call_card(env: &Envelope, call: &ToolCall) -> Hop {
    let label = call.label();
    Hop {
        from: env.to.clone(),
        to: label.clone(),
        hop: env.hop,
        kind: HopKind::Reply,
        text: format!("[tool] {}", call_line(&label, &call.args, 200)),
        usage: Default::default(),
    }
}

/// The card for a call's result: the `[tool]` marker and the outcome line,
/// exactly as the swarm builds them. Without the marker this card was not a
/// tool card to the app at all: the CALL was styled quiet and folded, and its
/// RESULT rendered beside it as a full, brightly-coloured agent reply. One
/// action, two looks, depending on which engine ran it.
pub(super) fn result_card(env: &Envelope, call: &ToolCall, ran: &Ran) -> Hop {
    let label = call.label();
    Hop {
        from: label.clone(),
        to: env.to.clone(),
        hop: env.hop,
        kind: HopKind::Reply,
        text: format!(
            "[tool] {}{}\n{}",
            result_line(&label, ran.ok, ran.ms),
            same_as(ran.repeat),
            clip_result(ran.text.trim_end(), RESULT_CLIP)
        ),
        usage: Default::default(),
    }
}

#[cfg(test)]
#[path = "toolbatch_tests.rs"]
mod tests;
