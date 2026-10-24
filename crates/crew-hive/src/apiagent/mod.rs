//! Native API agent: the swarm's worker, a task run as one harness turn
//! ([`crate::harness`]) on a [`Provider`] in an async future, emitting
//! telemetry events as it goes. The default headless scale worker.

#[cfg(test)]
#[path = "preamble_tests.rs"]
mod preamble_tests;
#[cfg(test)]
#[path = "samecall_tests.rs"]
mod samecall_tests;
#[cfg(test)]
#[path = "shrinkold_tests.rs"]
mod shrinkold_tests;
#[cfg(test)]
#[path = "stop_tests.rs"]
mod stop_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "todo_tests.rs"]
mod todo_tests;

mod context;
mod factory;

pub(crate) use context::build_prompt;
pub use factory::ApiFactory;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::agent::{Agent, AgentContext, Attempt};
use crate::board::TaskResult;
use crate::graph::{AgentKind, TaskId};
use crate::harness::{self, Ended, TurnSpec};
use crate::provider::Provider;
use crate::steers::Steers;
use crate::tools::{self, Tools};

pub struct ApiAgent {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// The tool surface, or `None` for a text-only agent. `None` must behave
    /// EXACTLY as this agent did before tools existed — same prompt bytes,
    /// same events, same single provider call — because that is still the
    /// configuration every keyless and mock run uses.
    tools: Option<Arc<dyn Tools>>,
    /// Standing context every task's system prompt ends with — the host's
    /// project card. A worker sees only its own task text, which the planner
    /// rewrote ("Define crew-term"), so without this it cannot know the
    /// thing it was asked about is a directory beside it.
    preamble: Option<String>,
    /// What the user typed into the run, read between rounds (`steered`).
    steers: Option<Steers>,
}

impl ApiAgent {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
            preamble: None,
            steers: None,
        }
    }

    /// Read what the user types into the run at every round (`steered`).
    pub fn with_steers(mut self, steers: Steers) -> Self {
        self.steers = Some(steers);
        self
    }

    /// End every task's system prompt with `preamble`.
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }

    /// Let this agent call tools between provider calls.
    pub fn with_tools(mut self, tools: Arc<dyn Tools>) -> Self {
        self.tools = Some(tools);
        self
    }
}

impl Agent for ApiAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let attempt = self.attempt(ctx);
        Box::pin(async move { attempt.await.result })
    }

    /// One task as one harness turn (`harness::run`): the task's system
    /// prompt and preamble, its dependencies' results in the prompt, a
    /// checklist of its own, and how the turn ended said as the scheduler
    /// hears it.
    fn attempt(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = Attempt> + Send>> {
        let provider = Arc::clone(&self.provider);
        let max_tokens = self.max_tokens;
        let model = self.model.clone();
        // Per task, so each worker keeps a checklist of its own (`todo`).
        let tools = self.tools.clone().map(tools::todo::per_task);
        let preamble = self.preamble.clone();
        let steers = self.steers.clone();
        Box::pin(async move {
            let task_id = ctx.task.id;
            // Honour the per-task model tier the planner assigned, not a fixed
            // factory tier — this is what lets a plan mix cheap and capable
            // models. The BILL follows the model id that actually answered
            // (`harness::billed`); the tier is only its fallback.
            let tier = ctx.task.model;
            let system = match &ctx.task.agent {
                AgentKind::Api { system } => system.clone(),
                AgentKind::Pty { .. } => None,
            };
            let spec = TurnSpec {
                ask: ctx.task.prompt.clone(),
                prompt: build_prompt(&ctx.task.prompt, &ctx.deps),
                system: with_preamble(system, preamble.as_deref()),
                model: model.unwrap_or_else(|| tier.model_id().to_owned()),
                tier,
                max_tokens,
            };
            let ended = harness::run(ctx.turn(), spec, provider, tools, steers).await;
            attempt_of(ended, task_id)
        })
    }
}

/// How a turn ended, as the scheduler hears it for `task`: an answer is a
/// success, a failure carries its reason as the output and whether it
/// passes, and a stop is recorded cancelled, never failed.
fn attempt_of(ended: Ended, task: TaskId) -> Attempt {
    match ended {
        Ended::Answered(output) => TaskResult {
            task,
            output,
            success: true,
        }
        .into(),
        Ended::Failed { why, transient } => Attempt {
            result: TaskResult {
                task,
                output: why,
                success: false,
            },
            transient,
            stopped: false,
        },
        Ended::Stopped => Attempt::stopped(task),
    }
}

/// `system` with `preamble` after it — or the preamble alone, or neither.
fn with_preamble(system: Option<String>, preamble: Option<&str>) -> Option<String> {
    match (system, preamble) {
        (s, None) => s,
        (None, Some(p)) => Some(p.to_string()),
        (Some(s), Some(p)) => Some(format!("{s}\n\n{p}")),
    }
}
