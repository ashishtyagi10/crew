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
    /// One streamed fragment of an agent's in-flight reply, published as it
    /// arrives from the provider. ADVISORY: the `OutputChunk` published when
    /// the agent finishes carries the COMPLETE output and is what the
    /// transcript keeps, so a subscriber that misses deltas — or ignores them
    /// entirely — loses liveness, never content.
    OutputDelta {
        agent: AgentId,
        text: String,
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
