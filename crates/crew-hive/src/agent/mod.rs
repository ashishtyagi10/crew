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

/// Everything an agent needs to do its task: its id, the task spec, the
/// already-gathered results of its dependencies, the event bus, and the
/// run's tool budget.
pub struct AgentContext {
    pub agent: AgentId,
    pub task: TaskSpec,
    pub deps: Vec<TaskResult>,
    pub bus: EventBus,
    /// The run's pool of tool rounds, shared with every other agent in it.
    pub budget: crate::tools::budget::ToolBudget,
}

/// A unit of work. Object-safe: `run` returns a boxed future so `Box<dyn Agent>`
/// works without the `async-trait` crate.
pub trait Agent: Send + Sync {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>>;
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
