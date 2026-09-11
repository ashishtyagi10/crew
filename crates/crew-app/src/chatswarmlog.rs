//! The LOG tee of a swarm run's telemetry (see [`crate::chatswarm`]): which
//! `Hive` events earn a line in the host's activity LOG, and its exact text.
//! Split from the live-status model so the two can grow apart — the status
//! line speaks task titles, the LOG speaks lifecycle, and a load (a skill
//! applied, a server connected) is lifecycle with no task of its own.
use crew_hive::{HiveEvent, TaskId, TaskState};

use crate::chatswarm::SwarmStatus;

/// The LOG line a hive event deserves, or `None` for the high-volume tiers
/// (token/cost/output deltas — liveness, not lifecycle). `(error, text)`;
/// titles come from the live plan so the LOG speaks task names, not ids.
#[cfg(test)]
pub(crate) fn log_line(swarm: Option<&SwarmStatus>, ev: &HiveEvent) -> Option<(bool, String)> {
    log_line_named(swarm, &std::collections::HashMap::new(), ev)
}

/// [`log_line`] with the pane's agent-id → name binding: a relay agent's
/// own tool calls (`claude` reading a file) say the agent's name, where a
/// hive agent — numbered by the run — says its number.
pub(crate) fn log_line_named(
    swarm: Option<&SwarmStatus>,
    names: &std::collections::HashMap<u64, String>,
    ev: &HiveEvent,
) -> Option<(bool, String)> {
    let title = |id: TaskId| -> String {
        swarm
            .and_then(|s| s.tasks.iter().find(|t| t.id == id))
            .map(|t| format!(" \u{2018}{}\u{2019}", t.title))
            .unwrap_or_default()
    };
    match ev {
        HiveEvent::AgentSpawned { agent, task } => Some((
            false,
            format!(
                "smith: agent {} took task #{}{}",
                agent.0,
                task.0,
                title(*task)
            ),
        )),
        HiveEvent::TaskStateChanged { task, state } => {
            let s = match state {
                TaskState::Pending => return None, // plan baseline, not news
                TaskState::Ready => return None,
                TaskState::Running => "running",
                TaskState::Done => "done",
                TaskState::Failed => "FAILED",
                TaskState::Cancelled => "cancelled",
            };
            Some((
                *state == TaskState::Failed,
                format!("smith: task #{}{} \u{2192} {s}", task.0, title(*task)),
            ))
        }
        HiveEvent::Failed { agent, error } => {
            Some((true, format!("smith: agent {} failed: {error}", agent.0)))
        }
        // A tool call IS lifecycle — it is the swarm touching something
        // outside crew — so it earns a LOG line where output deltas do not.
        HiveEvent::ToolCall { agent, label, .. } => {
            let who = names
                .get(&agent.0)
                .cloned()
                .unwrap_or_else(|| format!("agent {}", agent.0));
            Some((false, format!("smith: {who} called {label}")))
        }
        // Only failures: a successful call is already announced by its
        // ToolCall line, and repeating every one would double the volume of
        // the busiest thing a tool-using run does.
        HiveEvent::ToolResult {
            label, ok, text, ..
        } => (!ok).then(|| (true, format!("smith: {label} failed: {text}"))),
        // A load is lifecycle too — the run reaching for something outside
        // itself — said the way the pane's line says it.
        HiveEvent::Loaded {
            kind, name, detail, ..
        } => {
            // The detail's first word is the frame's verdict: the model
            // chose the playbook, or the task named it.
            let verb = match kind.as_str() {
                "skill" if detail.split_whitespace().next() == Some("chose") => "chosen",
                "skill" => "applied",
                "mcp" => "connected",
                _ => "started",
            };
            Some((false, format!("smith: {kind} {name} {verb}")))
        }
        HiveEvent::TokenDelta { .. }
        | HiveEvent::ToolBudget { .. }
        | HiveEvent::CostDelta { .. }
        | HiveEvent::OutputDelta { .. }
        | HiveEvent::ThoughtDelta { .. }
        | HiveEvent::OutputChunk { .. } => None,
    }
}

#[cfg(test)]
#[path = "chatswarmlog_tests.rs"]
mod tests;
