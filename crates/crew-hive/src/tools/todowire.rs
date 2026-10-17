//! An agent's checklist as the pane draws it: either spelling of the list,
//! crew's own `sys:todo` (`{"items": [{"text", "status"}]}`) or Claude Code's
//! TodoWrite (`{"todos": [{"content", "status", …}]}`), read into one shape
//! for the wire ([`Step`]).
//!
//! Lenient where [`super::todo`]'s parser is strict, on purpose. That parser
//! is a contract with the model: a list it refuses costs the model a round to
//! fix, and the refusal says how. This one only READS a list an agent has
//! already written. TodoWrite has its own rules, and a list Claude Code
//! accepted is the list its agent is working to, whatever its length.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One step of an agent's checklist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// What the step is, on one line.
    pub text: String,
    /// Where the agent says it is.
    pub state: StepState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Pending,
    Active,
    Done,
}

/// Steps one list carries over the wire. Past this a list is a backlog, and
/// the pane counts its tail rather than drawing it.
pub const MAX_STEPS: usize = 32;

/// Chars of a step's text the wire carries; the pane clips to its width.
pub const MAX_CHARS: usize = 160;

/// `args` (either spelling) as steps, or `None` when it holds no list.
pub fn steps(args: &str) -> Option<Vec<Step>> {
    let v: Value = serde_json::from_str(args.trim()).ok()?;
    let list = v.get("items").or_else(|| v.get("todos"))?.as_array()?;
    Some(list.iter().filter_map(step).take(MAX_STEPS).collect())
}

fn step(v: &Value) -> Option<Step> {
    let raw = v.get("text").or_else(|| v.get("content"))?.as_str()?;
    let text: String = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_CHARS)
        .collect();
    if text.is_empty() {
        return None;
    }
    let state = match v.get("status").and_then(Value::as_str) {
        Some("in_progress") => StepState::Active,
        Some("done" | "completed") => StepState::Done,
        _ => StepState::Pending,
    };
    Some(Step { text, state })
}

/// Whether a tool `label` is a checklist write: crew's own or Claude Code's.
pub fn is_list(label: &str) -> bool {
    matches!(label, "sys:todo" | "TodoWrite")
}

#[cfg(test)]
#[path = "todowire_tests.rs"]
mod tests;
