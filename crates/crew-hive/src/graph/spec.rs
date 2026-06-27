use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub struct TaskId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AgentKind {
    Pty { command: String, args: Vec<String> },
    Api { system: Option<String> },
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
