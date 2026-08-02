//! Blackboard: a concurrent shared store where agents publish their task
//! results and read upstream dependencies' results, so a fan-out of agents can
//! merge results upward. Cheap to clone (shared `Arc<RwLock<_>>`).
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::graph::TaskId;

/// The result an agent publishes for its task.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskResult {
    pub task: TaskId,
    pub output: String,
    pub success: bool,
}

#[derive(Default)]
struct Inner {
    results: HashMap<TaskId, TaskResult>,
    artifacts: HashMap<String, String>,
}

/// Shared, cloneable handle to the blackboard.
#[derive(Clone, Default)]
pub struct Blackboard {
    inner: Arc<RwLock<Inner>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn put_result(&self, result: TaskResult) {
        self.inner.write().await.results.insert(result.task, result);
    }

    pub async fn get_result(&self, task: TaskId) -> Option<TaskResult> {
        self.inner.read().await.results.get(&task).cloned()
    }

    /// Present results for `deps`, in the order given (absent ones skipped).
    ///
    /// Outputs are returned WHOLE — the board is the archive, and snapshot/
    /// artifact consumers need full text. Any prompt assembled from these
    /// must clip them to an explicit budget: see `apiagent::context`
    /// (`DEP_CAP` / `DEPS_TOTAL_CAP`), which `build_prompt` applies.
    pub async fn gather(&self, deps: &[TaskId]) -> Vec<TaskResult> {
        let g = self.inner.read().await;
        deps.iter()
            .filter_map(|d| g.results.get(d).cloned())
            .collect()
    }

    pub async fn result_count(&self) -> usize {
        self.inner.read().await.results.len()
    }

    pub async fn put_artifact(&self, key: impl Into<String>, value: impl Into<String>) {
        self.inner
            .write()
            .await
            .artifacts
            .insert(key.into(), value.into());
    }

    pub async fn get_artifact(&self, key: &str) -> Option<String> {
        self.inner.read().await.artifacts.get(key).cloned()
    }

    pub async fn snapshot(&self) -> BlackboardSnapshot {
        let g = self.inner.read().await;
        let mut results: Vec<TaskResult> = g.results.values().cloned().collect();
        results.sort_by_key(|r| r.task);
        let mut artifacts: Vec<(String, String)> = g
            .artifacts
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        artifacts.sort_by(|a, b| a.0.cmp(&b.0));
        BlackboardSnapshot { results, artifacts }
    }
}

/// A serializable point-in-time snapshot of the blackboard, for use across
/// the remote/sidecar bridge and swarm view.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlackboardSnapshot {
    pub results: Vec<TaskResult>,
    pub artifacts: Vec<(String, String)>,
}
