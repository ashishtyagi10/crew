//! Agent abstraction: a unit of work the scheduler runs. `Agent` is
//! object-safe (boxed future, no async-trait dep) so PTY/API/stub agents share
//! one interface. `AgentFactory` maps an `AgentKind` to a boxed agent.
mod stub;
#[cfg(test)]
mod tests;

pub(crate) mod stop;

pub use stub::StubAgent;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, TaskId, TaskSpec};
use crate::harness::TurnContext;
use crate::tools::budget::ToolBudget;

/// Everything an agent needs to do its task: its id, the task spec, the
/// already-gathered results of its dependencies, the event bus, the run's
/// tool budget, and the run's stop.
pub struct AgentContext {
    pub agent: AgentId,
    pub task: TaskSpec,
    pub deps: Vec<TaskResult>,
    pub bus: EventBus,
    /// The run's pool of tool rounds, shared with every other agent in it.
    /// Draw through [`TurnContext::take_round`], which announces the draw.
    pub budget: ToolBudget,
    /// The run's cancel flag, the scheduler's own (`Scheduler::with_cancel`).
    /// An agent that honours it stops at its next look and says so with
    /// [`Attempt::stopped`]; one that does not runs to its end, as every
    /// agent did before (see `stop`).
    pub cancel: Arc<AtomicBool>,
}

impl AgentContext {
    /// The harness turn this task runs as: its agent, publishing on the run's
    /// bus, drawing on the run's pool, stopped by the run's stop. No advice:
    /// the advisor is the relay's until it moves down (phase 4 of the goal).
    pub fn turn(&self) -> TurnContext {
        TurnContext {
            agent: self.agent.clone(),
            events: Arc::new(self.bus.clone()),
            budget: self.budget.clone(),
            cancel: Arc::clone(&self.cancel),
            advice: None,
        }
    }
}

/// A unit of work. Object-safe: `run` returns a boxed future so `Box<dyn Agent>`
/// works without the `async-trait` crate.
pub trait Agent: Send + Sync {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>>;

    /// [`Agent::run`], saying whether a failure passes — what the scheduler
    /// calls. Default: it never does. An agent that cannot tell a passing
    /// failure from a lasting one (a stub, a PTY process, a remote worker)
    /// must not earn its task a second run: for a process, that is doing the
    /// work twice on a guess.
    fn attempt(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = Attempt> + Send>> {
        let run = self.run(ctx);
        Box::pin(async move { run.await.into() })
    }
}

/// One run of a task as the scheduler sees it: the result, and whether its
/// failure is one that passes.
pub struct Attempt {
    pub result: TaskResult,
    /// The run failed in a way that passes — a read gone quiet, a dropped
    /// connection, a 429, a 5xx, an overload (`ProviderError::is_transient`)
    /// — so the same task, run again in a moment, may well succeed. The
    /// scheduler runs it ONCE more before anything re-plans
    /// (`sched::again`). Never set on a success.
    pub transient: bool,
    /// The agent stopped because the run was stopped, not because the task
    /// failed: the scheduler records it cancelled, never failed, so it earns
    /// no second run and no re-plan, and its half-done work is no answer.
    pub stopped: bool,
}

impl Attempt {
    /// The attempt of an agent that saw the run's stop and quit `task`.
    pub fn stopped(task: TaskId) -> Self {
        Self {
            result: TaskResult {
                task,
                output: "stopped".into(),
                success: false,
            },
            transient: false,
            stopped: true,
        }
    }
}

impl From<TaskResult> for Attempt {
    /// A result that says nothing about its failure: it lasts.
    fn from(result: TaskResult) -> Self {
        Self {
            result,
            transient: false,
            stopped: false,
        }
    }
}

/// Maps a task's `AgentKind` to a concrete agent.
pub trait AgentFactory: Send + Sync {
    fn make(&self, kind: &AgentKind) -> Box<dyn Agent>;
}

/// Test factory: makes always-succeeding stub agents.
pub struct StubFactory;

impl AgentFactory for StubFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(StubAgent {
            fail_ids: HashSet::new(),
        })
    }
}

/// Test factory: makes stub agents that fail for the configured task ids.
pub struct FailingFactory {
    pub fail_tasks: HashSet<TaskId>,
}

impl AgentFactory for FailingFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(StubAgent {
            fail_ids: self.fail_tasks.clone(),
        })
    }
}
