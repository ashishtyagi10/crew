//! How many swarm tasks run at once — the plan's own width, not a constant.
//!
//! WHY: the scheduler took a fixed `4` for every run. The planner's prompt
//! has a `deps` clause whose whole point is to make plans WIDE (independent
//! tasks side by side), and then the scheduler capped a six-wide plan at four
//! and ran a two-wide plan with two idle permits. The plan already says how
//! parallel the work is: its initial ready set — the tasks with no
//! dependencies — is exactly the width the first wave wants. So that is the
//! concurrency, clamped so a one-task plan still has a permit for the merge a
//! re-plan may add ([`MIN`]) and a twelve-wide plan does not fire twelve
//! requests at one provider at once ([`MAX`]).
//!
//! `CREW_SWARM_CONCURRENCY=<n>` is the one override, read in one place
//! ([`concurrency_for`]) and clamped to `1..=16`. It is a rate-limit knob as
//! much as a speed knob: every permit is one request in flight against the
//! provider at the same moment, so a value above the provider's per-minute
//! allowance turns a wide plan into a run of 429s, and `1` makes any plan
//! run serially for a debugging session or a tight allowance.
use std::collections::HashSet;

use crew_hive::TaskGraph;

/// The narrowest a run goes on its own: a permit for the task and one for
/// what a re-plan adds beside it.
pub(crate) const MIN: usize = 2;
/// The widest a run goes on its own — the plan may be wider; the provider's
/// rate limit is not.
pub(crate) const MAX: usize = 8;
/// The widest the override may ask for.
pub(crate) const OVERRIDE_MAX: usize = 16;

/// The concurrency for `graph`: the override when `CREW_SWARM_CONCURRENCY`
/// holds a number, else the plan's initial width clamped to `MIN..=MAX`.
pub(super) fn concurrency_for(graph: &TaskGraph) -> usize {
    let width = graph.ready(&HashSet::new()).len();
    concurrency(
        width,
        std::env::var("CREW_SWARM_CONCURRENCY").ok().as_deref(),
    )
}

/// [`concurrency_for`] on its inputs: `width` is the plan's initial ready
/// count, `raw` the override's text (a number, clamped to `1..=OVERRIDE_MAX`;
/// anything else is ignored, never a zero).
pub(crate) fn concurrency(width: usize, raw: Option<&str>) -> usize {
    let forced = raw
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|n| *n > 0);
    match forced {
        Some(n) => n.min(OVERRIDE_MAX),
        None => width.clamp(MIN, MAX),
    }
}

#[cfg(test)]
#[path = "swarmwidth_tests.rs"]
mod tests;
