//! Fleet telemetry: per-agent snapshot built by applying `HiveEvent`s.
//! The swarm view renders `Fleet` to show live progress, token costs, etc.

use crate::bus::{AgentId, HiveEvent};
use crate::graph::{TaskId, TaskState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// Live counters and status for a single spawned agent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentTelemetry {
    pub agent: AgentId,
    pub task: TaskId,
    pub state: TaskState,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub micros_usd: u64,
    pub last_line: String,
}

/// Fleet-wide aggregate counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetTotals {
    pub live: usize,
    pub done: usize,
    pub failed: usize,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub micros_usd: u64,
}

/// Snapshot of all agents, keyed by `AgentId.0` for ascending iteration.
#[derive(Debug, Default)]
pub struct Fleet {
    agents: BTreeMap<u64, AgentTelemetry>,
}

impl Fleet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a `HiveEvent` to update the fleet snapshot.
    ///
    /// `AgentSpawned` inserts a new record (state `Running`). All other
    /// events for unknown agents are silently ignored.
    pub fn apply(&mut self, ev: &HiveEvent) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.agents.insert(
                    agent.0,
                    AgentTelemetry {
                        agent: agent.clone(),
                        task: *task,
                        state: TaskState::Running,
                        tokens_in: 0,
                        tokens_out: 0,
                        micros_usd: 0,
                        last_line: String::new(),
                    },
                );
            }
            HiveEvent::TokenDelta {
                agent,
                input,
                output,
            } => {
                if let Some(rec) = self.agents.get_mut(&agent.0) {
                    rec.tokens_in = rec.tokens_in.saturating_add(*input);
                    rec.tokens_out = rec.tokens_out.saturating_add(*output);
                }
            }
            HiveEvent::CostDelta { agent, micros_usd } => {
                if let Some(rec) = self.agents.get_mut(&agent.0) {
                    rec.micros_usd = rec.micros_usd.saturating_add(*micros_usd);
                }
            }
            HiveEvent::OutputChunk { agent, text } => {
                if let Some(rec) = self.agents.get_mut(&agent.0) {
                    // Last non-empty line (after trimming whitespace); if all
                    // lines are empty, leave last_line unchanged.
                    if let Some(line) = text.lines().rfind(|l| !l.trim().is_empty()) {
                        rec.last_line = line.to_string();
                    }
                }
            }
            HiveEvent::Failed { agent, error } => {
                if let Some(rec) = self.agents.get_mut(&agent.0) {
                    rec.state = TaskState::Failed;
                    rec.last_line = error.clone();
                }
            }
            HiveEvent::TaskStateChanged { task, state } => {
                // Update the first agent whose task matches; duplicate task
                // assignments are not expected but we handle gracefully.
                for rec in self.agents.values_mut() {
                    if rec.task == *task {
                        rec.state = *state;
                        break;
                    }
                }
            }
        }
    }

    /// Iterate agents in ascending agent-id order.
    pub fn agents(&self) -> impl Iterator<Item = &AgentTelemetry> {
        self.agents.values()
    }

    /// Look up a single agent's telemetry.
    pub fn get(&self, agent: AgentId) -> Option<&AgentTelemetry> {
        self.agents.get(&agent.0)
    }

    /// Number of agents in the fleet.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// True when no agents have been spawned.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Aggregate counts across all agents.
    pub fn totals(&self) -> FleetTotals {
        let mut t = FleetTotals {
            live: 0,
            done: 0,
            failed: 0,
            tokens_in: 0,
            tokens_out: 0,
            micros_usd: 0,
        };
        for rec in self.agents.values() {
            match rec.state {
                TaskState::Running => t.live += 1,
                TaskState::Done => t.done += 1,
                TaskState::Failed => t.failed += 1,
                _ => {}
            }
            t.tokens_in = t.tokens_in.saturating_add(u64::from(rec.tokens_in));
            t.tokens_out = t.tokens_out.saturating_add(u64::from(rec.tokens_out));
            t.micros_usd = t.micros_usd.saturating_add(rec.micros_usd);
        }
        t
    }
}
