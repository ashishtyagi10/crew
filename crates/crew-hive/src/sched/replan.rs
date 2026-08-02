//! Mid-run re-planning: the graph unfreezes on failure. When a task fails,
//! the scheduler pauses ready-dispatch and asks the planner — one bounded
//! call, the same [`Planner`] the run started with — for a REPLACEMENT
//! sub-graph covering the not-yet-run remainder, given the goal, the
//! completed outputs (budgeted through `apiagent::context`, never an
//! unbounded concat) and the failure. Completed tasks and their outputs are
//! never re-run; a planner error (or a keyless/mock run, which never gets a
//! replanner) keeps today's cascade-cancel untouched.
//!
//! SECURITY INVARIANT: replacement tasks pass through the same forcing as
//! `planner::parse_plan` — every one is [`AgentKind::Api`] at
//! [`ModelTier::Standard`], whatever the (model-authored, untrusted) plan
//! asked for. A re-plan must never widen what a plan can execute.
use std::collections::HashSet;

use crate::board::Blackboard;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::Planner;

/// Re-plans allowed per run — a hard constant, not a knob: one failure gets
/// one fresh look at the remainder; a second failure cascade-cancels as
/// before, so a flaky graph cannot loop the planner.
pub(super) const REPLAN_CAP: usize = 1;

/// What the scheduler needs to re-plan: the run's goal and its planner.
pub(super) struct Replan {
    pub goal: String,
    pub planner: std::sync::Arc<dyn Planner>,
}

/// The scheduler's failure hook: ask for a replacement remainder and — when
/// one arrives — mark the stale not-yet-run tasks Cancelled (superseded, not
/// failed) and swap `graph` for it. On a planner error nothing changes and
/// the failure cascade-cancels exactly as before. The scheduler awaits this
/// before its next spawn pass, so ready-dispatch is paused throughout.
#[allow(clippy::too_many_arguments)] // the scheduler loop's working set, borrowed piecewise
pub(super) async fn attempt(
    rp: &Replan,
    graph: &mut TaskGraph,
    board: &Blackboard,
    bus: &crate::bus::EventBus,
    done: &HashSet<TaskId>,
    failed: &HashSet<TaskId>,
    cancelled: &mut HashSet<TaskId>,
    started: &HashSet<TaskId>,
    failed_task: TaskId,
    failure: &str,
) {
    if let Some(g2) = replacement(rp, graph, done, failed_task, failure, board).await {
        super::cancel::mark_all_unstarted_cancelled(graph, bus, done, failed, cancelled, started);
        *graph = g2;
    }
}

/// Ask the planner for a replacement sub-graph after `failed_task` failed.
/// `None` on any planner/graph error — the caller falls back to
/// cascade-cancel. Replacement task ids are remapped past the old graph's
/// maximum so they can never collide with (or resurrect) an old task.
pub(super) async fn replacement(
    rp: &Replan,
    graph: &TaskGraph,
    done: &HashSet<TaskId>,
    failed_task: TaskId,
    failure: &str,
    board: &Blackboard,
) -> Option<TaskGraph> {
    let completed = board.gather(&done_sorted(done)).await;
    let failed_title = graph
        .get(failed_task)
        .map(|t| t.title.clone())
        .unwrap_or_default();
    // The failure rides as the LAST context entry so it shares the same
    // per-dep/total budgeting as the completed outputs.
    let mut context = completed;
    context.push(crate::board::TaskResult {
        task: failed_task,
        output: format!("FAILED task \u{201c}{failed_title}\u{201d}: {failure}"),
        success: false,
    });
    let header = format!(
        "REPLAN: the goal below is partway done. Task \u{201c}{failed_title}\u{201d} \
         failed (its error is the last context entry; completed task outputs \
         precede it). Plan ONLY the remaining work as a fresh task array \u{2014} \
         do not repeat completed work.\n\nGoal: {}",
        rp.goal
    );
    let prompt = crate::apiagent::build_prompt(&header, &context);
    let plan = rp.planner.plan(&prompt).await.ok()?;
    let offset = graph.tasks().iter().map(|t| t.id.0).max().unwrap_or(0) + 1;
    let specs: Vec<TaskSpec> = plan
        .tasks()
        .iter()
        .map(|t| TaskSpec {
            id: TaskId(t.id.0 + offset),
            title: t.title.clone(),
            agent: force_api(&t.agent),
            model: ModelTier::Standard,
            deps: t.deps.iter().map(|d| TaskId(d.0 + offset)).collect(),
            prompt: t.prompt.clone(),
            specialty: t.specialty.clone(),
            expertise: t.expertise.clone(),
        })
        .collect();
    debug_assert!(
        !specs.iter().any(|t| t.agent.is_pty()),
        "a re-plan must never yield a process-executing Pty task"
    );
    TaskGraph::new(specs).ok()
}

/// The `parse_plan` forcing, applied again at the trust boundary: a re-plan
/// arrives as a [`TaskGraph`] (any [`Planner`] impl), so the scheduler
/// cannot assume it already went through `parse_plan`.
fn force_api(kind: &AgentKind) -> AgentKind {
    match kind {
        AgentKind::Api { system } => AgentKind::Api {
            system: system.clone(),
        },
        AgentKind::Pty { .. } => AgentKind::Api { system: None },
    }
}

/// `done` as a sorted Vec — deterministic context order for the planner.
fn done_sorted(done: &HashSet<TaskId>) -> Vec<TaskId> {
    let mut v: Vec<TaskId> = done.iter().copied().collect();
    v.sort_unstable();
    v
}

#[cfg(test)]
#[path = "replan_tests.rs"]
mod tests;
