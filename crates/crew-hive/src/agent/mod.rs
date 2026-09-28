//! Agent abstraction: a unit of work the scheduler runs. `Agent` is
//! object-safe (boxed future, no async-trait dep) so PTY/API/stub agents share
//! one interface. `AgentFactory` maps an `AgentKind` to a boxed agent.
mod stub;
#[cfg(test)]
mod tests;

pub use stub::StubAgent;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, TaskId, TaskSpec};
use crate::tools::budget::ToolBudget;

/// Everything an agent needs to do its task: its id, the task spec, the
/// already-gathered results of its dependencies, the event bus, and the
/// run's tool budget.
pub struct AgentContext {
    pub agent: AgentId,
    pub task: TaskSpec,
    pub deps: Vec<TaskResult>,
    pub bus: EventBus,
    /// The run's pool of tool rounds, shared with every other agent in it.
    /// Draw through [`AgentContext::take_round`], which announces the draw.
    pub budget: ToolBudget,
}

impl AgentContext {
    /// Draw one tool round from the run's pool for an agent that has drawn
    /// `used` already this task (see [`ToolBudget::take`]), and publish the
    /// pool's state so the run can watch it drain. The one way an agent
    /// spends a round: a draw nobody announced is a pool nobody can show.
    pub fn take_round(&self, used: u32) -> Option<u32> {
        let left = self.budget.take(used);
        let total = self.budget.total();
        self.bus.publish(crate::bus::HiveEvent::ToolBudget {
            used: total - self.budget.left(),
            total,
        });
        left
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
}

impl From<TaskResult> for Attempt {
    /// A result that says nothing about its failure: it lasts.
    fn from(result: TaskResult) -> Self {
        Self {
            result,
            transient: false,
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
