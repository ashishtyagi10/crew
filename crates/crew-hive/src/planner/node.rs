//! One task of a plan as the model wrote it, and its id and deps read the
//! ways models actually write them.
//!
//! The prompt asks for an integer `id` and a `deps` array on every task, and
//! the parser used to hold the model to exactly that: `"id": "1"`, a root
//! task with no `deps`, `"deps": ["1"]` or `"depends_on"` each failed the
//! parse, and the planner was asked again, a whole planning call spent on a
//! plan that meant exactly what it said. Those spellings are read here now.
//! What stays refused is what the graph cannot mean: an id with no number in
//! it, two tasks with one id, a dep on a task that is not there, a cycle
//! (the last three are `TaskGraph::new`'s to refuse, and still are).

use serde::Deserialize;
use serde_json::Value;

use super::PlanError;

/// The shape we accept from model output. Deliberately has **no** `agent`,
/// `command`, or `args` field: the model describes *what* work to do and *who*
/// should do it, never *how* to execute it. serde ignores any such extra keys,
/// so an attacker-influenced completion cannot smuggle one in. See the
/// security note on `parse_plan`.
#[derive(Deserialize)]
pub(super) struct PlanNode {
    /// A number or a string; read by [`task_id`].
    id: Value,
    pub(super) title: String,
    pub(super) prompt: String,
    /// Missing or `null` on a root task means no deps: that is what a model
    /// leaves out when there are none, and there is nothing else it could
    /// mean. `depends_on` and `dependencies` are the names models reach for
    /// when they forget ours.
    #[serde(default, alias = "depends_on", alias = "dependencies")]
    deps: Option<Vec<Value>>,
    #[serde(default)]
    pub(super) specialty: Option<String>,
    #[serde(default)]
    pub(super) expertise: Option<String>,
}

impl PlanNode {
    /// This task's id, then the ids it waits on, each read by [`task_id`].
    pub(super) fn ids(&self) -> Result<(u64, Vec<u64>), PlanError> {
        let id = task_id(&self.id).map_err(|e| PlanError::Parse(format!("task id {e}")))?;
        let deps = self
            .deps
            .iter()
            .flatten()
            .map(|d| task_id(d).map_err(|e| PlanError::Parse(format!("task {id}'s dep {e}"))))
            .collect::<Result<_, _>>()?;
        Ok((id, deps))
    }
}

/// An id as a model writes one: a whole number, as a number or a string
/// (`1`, `"1"`), or a string of letters then that number (`"t1"`,
/// `"task-2"`, `"step_3"`), which is read as the number. Ids and deps are read
/// the same way, so a plan that says `"t1"` everywhere means the same graph
/// as one that says `1`; two ids that come to one number (`"a1"`, `"b1"`)
/// are a duplicate, and the graph refuses them as one. Anything else — a
/// word, a fraction, a negative, `"1a"` — is not an id, and the plan is
/// asked for again.
pub(super) fn task_id(v: &Value) -> Result<u64, String> {
    let n = match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => from_text(s.trim()),
        _ => None,
    };
    n.ok_or_else(|| format!("{v} is not a whole number"))
}

/// The number in `"12"` or `"t12"`, or `None`.
fn from_text(s: &str) -> Option<u64> {
    let digits = s.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    // A separator only after a word: "-1" is a negative, not "task -1".
    let digits = match digits.len() < s.len() {
        true => digits.strip_prefix(['-', '_']).unwrap_or(digits),
        false => digits,
    };
    // `u64::from_str` would take a leading `+`; an id never has one.
    let whole = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    whole.then(|| digits.parse().ok()).flatten()
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
