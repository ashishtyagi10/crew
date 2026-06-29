use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub struct TaskId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AgentKind {
    Pty { command: String, args: Vec<String> },
    Api { system: Option<String> },
}

impl AgentKind {
    /// Whether this kind spawns an OS process (`command`/`args`). This is the
    /// security-sensitive variant: a task graph derived from untrusted input
    /// (e.g. an LLM-produced plan) must never carry a `Pty` agent, or the
    /// command/args become a command-injection sink once a Pty executor exists.
    /// See `planner::parse_plan`, which forces every model-derived task to `Api`.
    pub fn is_pty(&self) -> bool {
        matches!(self, AgentKind::Pty { .. })
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelTier {
    Cheap,
    Standard,
    Capable,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: TaskId,
    pub title: String,
    pub agent: AgentKind,
    pub model: ModelTier,
    pub deps: Vec<TaskId>,
    pub prompt: String,
}

#[derive(Debug, PartialEq)]
pub enum GraphError {
    DuplicateId(TaskId),
    MissingDep { task: TaskId, dep: TaskId },
    Cycle,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::DuplicateId(id) => write!(f, "duplicate task id: {}", id.0),
            GraphError::MissingDep { task, dep } => {
                write!(f, "task {} depends on missing task {}", task.0, dep.0)
            }
            GraphError::Cycle => write!(f, "task graph contains a cycle"),
        }
    }
}

impl std::error::Error for GraphError {}

impl ModelTier {
    /// The default Anthropic model id for this cost tier.
    pub fn model_id(&self) -> &'static str {
        match self {
            ModelTier::Cheap => "claude-haiku-4-5",
            ModelTier::Standard => "claude-sonnet-4-6",
            ModelTier::Capable => "claude-opus-4-8",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_id_cheap() {
        assert_eq!(ModelTier::Cheap.model_id(), "claude-haiku-4-5");
    }

    #[test]
    fn model_id_standard() {
        assert_eq!(ModelTier::Standard.model_id(), "claude-sonnet-4-6");
    }

    #[test]
    fn model_id_capable() {
        assert_eq!(ModelTier::Capable.model_id(), "claude-opus-4-8");
    }
}
