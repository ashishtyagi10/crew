//! Stopping an agent part-way through its task.
//!
//! The run's cancel flag used to be read by the scheduler alone: it marked
//! the tasks not yet started as cancelled and then waited for every running
//! agent to finish by itself. A worker mid-way through its tool loop, up to
//! eight rounds of model calls and tool runs, went on calling and billing
//! after the person had said stop, and the stop waited for all of it. The
//! flag now rides in the agent's context (`AgentContext::cancel`): a worker
//! looks before each model call and each tool call, and races a model call
//! in flight against it.
//!
//! LOOKED AT, NOT WOKEN. The flag is a plain `AtomicBool` that several
//! parties set (`/stop`, the budget governor, the host's own cancel), and a
//! waiter that must be woken would need every one of them changed. A waiting
//! agent looks every [`TICK`] instead: a stop lands within one tick, for
//! twenty cheap wake-ups a second while a call is out.
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How often an agent waiting on a call looks at the flag.
pub(crate) const TICK: Duration = Duration::from_millis(50);

/// Resolves once `flag` is set.
pub(crate) async fn stopped(flag: &AtomicBool) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(TICK).await;
    }
}

/// `work`'s output, or `None` when `flag` is set first. The work is dropped
/// then, unfinished: for a provider call that aborts the request itself
/// (`provider::io::Joined`), so nothing streams or bills after the stop.
/// A flag already set wins without `work` being polled at all.
pub(crate) async fn unless<F: Future>(flag: &AtomicBool, work: F) -> Option<F::Output> {
    tokio::select! {
        biased;
        () = stopped(flag) => None,
        out = work => Some(out),
    }
}

#[cfg(test)]
#[path = "stop_tests.rs"]
mod tests;
