//! What a scheduler run leaves behind: which tasks ended how, and how much
//! of the run's tool pool they spent.
//!
//! The pool (`tools::budget::ToolBudget`) is built inside `Scheduler::run`
//! and handed to the agents; nothing outside could read it, so a host that
//! wanted to say `tools 5/12` had to guess. The outcome carries the final
//! count out — the scheduler is the one party that saw every draw.
use crate::graph::TaskId;

#[derive(Clone, Debug, PartialEq)]
pub struct RunOutcome {
    pub done: Vec<TaskId>,
    pub failed: Vec<TaskId>,
    pub cancelled: Vec<TaskId>,
    /// `(used, total)` tool rounds over the whole run. `total` is never 0
    /// for a run the scheduler sized (four per task, one task minimum).
    pub tool_rounds: (u32, u32),
}

#[cfg(test)]
#[path = "outcome_tests.rs"]
mod tests;
