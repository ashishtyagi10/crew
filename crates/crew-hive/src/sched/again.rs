//! One more run for a task whose failure passes.
//!
//! Any failure used to go straight to the planner: a whole planning call and
//! a new graph, for a 429 or a dropped connection that the provider's own
//! retries (`provider::retry`, seconds apart) happened not to outlast. A
//! failure that passes — the agent says so in its [`Attempt`], from
//! `ProviderError::is_transient` — now gets its task run ONCE more after a
//! short pause, and a second run that succeeds is simply done. One that
//! lasts (a rejected key, a model the host does not serve, a 400) goes to
//! the planner at once, as before, with no second run to pay for.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::FutureExt as _;

use crate::agent::{Agent, AgentContext, Attempt};
use crate::board::TaskResult;

/// How long a task waits before its second run: long enough for a
/// rate-limit window or a restarting host to turn over, short enough that a
/// person watching the pane reads it as a stumble, not a stall.
pub(super) const PAUSE: Duration = Duration::from_millis(1500);

/// Run the task; when that fails in a way that passes, wait `pause` and run
/// it once more, and let the second result stand whatever it is. A stop
/// pressed during the pause is honoured — the agent is not running then, so
/// nothing makes the task un-cancellable yet — and the first failure stands.
///
/// `None` when the agent quit because the run was stopped
/// ([`Attempt::stopped`]): the scheduler records that as cancelled, as it
/// does a task stopped before its agent started. It is no failure, so it
/// earns neither the second run here nor the re-plan a failure gets.
pub(super) async fn run(
    agent: &dyn Agent,
    ctx: AgentContext,
    pause: Duration,
    cancel: &AtomicBool,
) -> Option<TaskResult> {
    let second = AgentContext {
        cancel: ctx.cancel.clone(),
        agent: ctx.agent.clone(),
        task: ctx.task.clone(),
        deps: ctx.deps.clone(),
        bus: ctx.bus.clone(),
        budget: ctx.budget.clone(),
    };
    let first = guarded(agent, ctx).await;
    if first.stopped {
        return None;
    }
    if !worth_another(&first) {
        return Some(first.result);
    }
    tokio::time::sleep(pause).await;
    if cancel.load(Ordering::Relaxed) {
        return Some(first.result);
    }
    let again = guarded(agent, second).await;
    (!again.stopped).then_some(again.result)
}

/// A failure that passes. A success never runs again, and neither does a
/// failure that would fail the same way the second time.
fn worth_another(a: &Attempt) -> bool {
    !a.result.success && a.transient
}

/// One run, with a panic turned into a failed result rather than an abort
/// of the whole swarm — and a panic never passes.
async fn guarded(agent: &dyn Agent, ctx: AgentContext) -> Attempt {
    let task = ctx.task.id;
    match std::panic::AssertUnwindSafe(agent.attempt(ctx))
        .catch_unwind()
        .await
    {
        Ok(a) => a,
        Err(_) => TaskResult {
            task,
            output: "agent panicked".into(),
            success: false,
        }
        .into(),
    }
}

#[cfg(test)]
#[path = "again_tests.rs"]
mod tests;
