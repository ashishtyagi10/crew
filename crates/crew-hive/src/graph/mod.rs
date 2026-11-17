//! Task DAG: specs, dependency readiness, and validation.
mod spec;
#[cfg(test)]
mod tests;

pub use spec::{AgentKind, GraphError, ModelTier, TaskId, TaskSpec, TaskState};

use std::collections::{HashMap, HashSet, VecDeque};

/// Maximum number of tasks accepted in one plan.
pub const MAX_TASKS: usize = 256;

#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: Vec<TaskSpec>,
}

impl TaskGraph {
    pub fn new(tasks: Vec<TaskSpec>) -> Result<Self, GraphError> {
        if tasks.len() > MAX_TASKS {
            return Err(GraphError::TooManyTasks {
                max: MAX_TASKS,
                actual: tasks.len(),
            });
        }
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

    /// Kahn's algorithm in O(tasks + dependencies), independent of plan shape.
    fn has_cycle(&self) -> bool {
        let indices: HashMap<TaskId, usize> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| (task.id, i))
            .collect();
        let mut incoming: Vec<usize> = self.tasks.iter().map(|task| task.deps.len()).collect();
        let mut outgoing = vec![Vec::new(); self.tasks.len()];
        for (i, task) in self.tasks.iter().enumerate() {
            for dep in &task.deps {
                outgoing[indices[dep]].push(i);
            }
        }
        let mut ready: VecDeque<usize> = incoming
            .iter()
            .enumerate()
            .filter_map(|(i, &n)| (n == 0).then_some(i))
            .collect();
        let mut removed = 0;
        while let Some(i) = ready.pop_front() {
            removed += 1;
            for &next in &outgoing[i] {
                incoming[next] -= 1;
                if incoming[next] == 0 {
                    ready.push_back(next);
                }
            }
        }
        removed != self.tasks.len()
    }
}
