//! The crew pane's pulse tracking: per-agent hop timings observed from
//! `Activity`/reply events. `hops` feeds the session line's turn-duration
//! figure — the sum of the settled turn's hop times.
use std::collections::HashMap;

use crate::spark::History;

/// Hop-duration samples kept per agent (sparkline window and then some).
const HIST_CAP: usize = 32;

/// Per-agent hop timings observed live from `Activity`/reply events: the
/// per-agent hop-duration history and the recorded hops for the in-flight
/// (or last completed) turn.
pub(crate) struct Pulse {
    hist: HashMap<String, History>,
    hops: Vec<(String, u64)>,
    turn_done: bool,
}

impl Pulse {
    pub(crate) fn new() -> Self {
        Pulse {
            hist: HashMap::new(),
            hops: Vec::new(),
            turn_done: false,
        }
    }

    /// A hop is starting: a settled turn's hop list resets so the new turn
    /// starts from a clean list; mid-turn this is a no-op.
    pub(crate) fn begin_hop(&mut self) {
        if self.turn_done {
            self.hops.clear();
            self.turn_done = false;
        }
    }

    /// A hop finished after `ms`: append to the hop list and the agent's
    /// sparkline history.
    pub(crate) fn record_hop(&mut self, agent: &str, ms: u64) {
        self.hist
            .entry(agent.to_string())
            .or_insert_with(|| History::new(HIST_CAP))
            .push(ms);
        self.hops.push((agent.to_string(), ms));
    }

    /// The turn is over: freeze the hop list as the settled record until the
    /// next turn's first hop.
    pub(crate) fn end_turn(&mut self) {
        self.turn_done = true;
    }

    /// Per-agent hop-duration history — kept for its own tested contract
    /// (`record_hop` accumulation, capped window, surviving turn resets)
    /// even though the lane sparkline that used to render it is gone; a
    /// future per-agent trend view is the natural consumer.
    #[allow(dead_code)]
    pub(crate) fn hist(&self, agent: &str) -> Option<&History> {
        self.hist.get(agent)
    }

    /// The settled turn's recorded hops. Its header turn-duration consumer
    /// was removed in the reductionist panel cleanup (one counter only), but
    /// the begin/record/end-turn accumulation keeps its tested contract for a
    /// future per-turn trend view — same rationale as [`Self::hist`].
    #[allow(dead_code)]
    pub(crate) fn hops(&self) -> &[(String, u64)] {
        &self.hops
    }
}

#[cfg(test)]
#[path = "chatpulse_tests.rs"]
mod tests;
