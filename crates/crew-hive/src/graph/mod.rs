//! Task DAG: specs, dependency readiness, and validation.
mod spec;
#[cfg(test)]
mod tests;

pub use spec::{AgentKind, GraphError, ModelTier, TaskId, TaskSpec, TaskState};

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: Vec<TaskSpec>,
}

impl TaskGraph {
    pub fn new(tasks: Vec<TaskSpec>) -> Result<Self, GraphError> {
        let mut ids = HashSet::new();
        for t in &tasks {
            if !ids.insert(t.id) {
                return Err(GraphError::DuplicateId(t.id));
            }
        }
        for t in &tasks {
            for d in &t.deps {
                if !ids.contains(d) {
                    return Err(GraphError::MissingDep {
                        task: t.id,
                        dep: *d,
                    });
                }
            }
        }
        let g = Self { tasks };
        if g.has_cycle() {
            return Err(GraphError::Cycle);
        }
        Ok(g)
    }

    pub fn tasks(&self) -> &[TaskSpec] {
        &self.tasks
    }

    pub fn get(&self, id: TaskId) -> Option<&TaskSpec> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn ready(&self, done: &HashSet<TaskId>) -> Vec<TaskId> {
        let mut out: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|t| !done.contains(&t.id) && t.deps.iter().all(|d| done.contains(d)))
            .map(|t| t.id)
            .collect();
        out.sort_unstable();
        out
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Kahn's algorithm: if we cannot remove all nodes by repeatedly removing
    /// those with no unsatisfied deps, a cycle exists.
    fn has_cycle(&self) -> bool {
        let mut done: HashSet<TaskId> = HashSet::new();
        loop {
            let next = self.ready(&done);
            if next.is_empty() {
                return done.len() != self.tasks.len();
            }
            for id in next {
                done.insert(id);
            }
        }
    }
}
