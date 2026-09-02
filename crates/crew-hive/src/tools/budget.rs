//! Tool rounds as a budget over the RUN, not a cap per task.
//!
//! [`MAX_TOOL_ROUNDS`] was sized for a world with four tools: four rounds
//! per task, and a wrong first pick cost a quarter of them. With a catalog
//! of two hundred, a task that needs six rounds and a task that needs one
//! are both common, and a per-task cap starves the first to protect the
//! second. So the rounds are pooled: the run gets four per task, any agent
//! may draw up to twice its old share, and what one task leaves is there for
//! another. A single-task run is exactly what it was.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::MAX_TOOL_ROUNDS;

/// The pool one run's agents draw tool rounds from.
#[derive(Debug, Clone)]
pub struct ToolBudget {
    left: Arc<AtomicU32>,
    total: u32,
    /// Most rounds ONE agent may take, however full the pool: a swarm of
    /// twelve must not lose its whole allowance to the first agent that
    /// loops.
    per_agent: u32,
}

impl Default for ToolBudget {
    fn default() -> Self {
        Self::solo()
    }
}

impl ToolBudget {
    /// The pool for a run of `tasks` tasks.
    pub fn for_run(tasks: usize) -> Self {
        let total = MAX_TOOL_ROUNDS * u32::try_from(tasks.max(1)).unwrap_or(u32::MAX);
        Self {
            left: Arc::new(AtomicU32::new(total)),
            total,
            per_agent: MAX_TOOL_ROUNDS * 2,
        }
    }

    /// One task on its own: the old cap, exactly.
    pub fn solo() -> Self {
        Self::for_run(1)
    }

    /// Take one round for an agent that has taken `used` already this task.
    /// `Some(n)` is how many more it may take after this one; `None` means
    /// the pool, or this agent's own ceiling, is spent — and nothing was
    /// taken.
    pub fn take(&self, used: u32) -> Option<u32> {
        if used >= self.per_agent {
            return None;
        }
        let left = self
            .left
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1))
            .ok()?
            - 1;
        Some(left.min(self.per_agent - used - 1))
    }

    /// Rounds the whole run was given.
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Rounds nobody has taken yet.
    pub fn left(&self) -> u32 {
        self.left.load(Ordering::Acquire)
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
