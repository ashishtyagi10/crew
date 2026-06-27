use crate::graph::{TaskId, TaskState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HiveEvent {
    TaskStateChanged {
        task: TaskId,
        state: TaskState,
    },
    AgentSpawned {
        agent: AgentId,
        task: TaskId,
    },
    TokenDelta {
        agent: AgentId,
        input: u32,
        output: u32,
    },
    CostDelta {
        agent: AgentId,
        micros_usd: u64,
    },
    OutputChunk {
        agent: AgentId,
        text: String,
    },
    Failed {
        agent: AgentId,
        error: String,
    },
}
