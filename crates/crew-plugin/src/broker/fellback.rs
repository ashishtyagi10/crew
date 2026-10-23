//! Agent smith's word that a model chain fell through.
//!
//! WHY: a provider's chain (`CREW_DASHSCOPE_MODEL="qwen-max,qwen-flash"`)
//! moved to its next model on any failure and said nothing, so the model
//! asked for could be gone for good while every reply, and its bill, came
//! from another. The provider now records what it skipped
//! (`Completion::fell_back`), and smith says so under the agent's reply —
//! ONCE per agent a turn: a chain whose first model is gone falls through on
//! every call, and a line per call would bury the answers between them.
use super::adapter::Usage;
use super::relay::msg;
use crate::PluginEvent;

/// `qwen-flash · qwen-max: <why>`: the model that answered, then what its
/// chain skipped to reach it; empty when nothing was skipped.
pub(crate) fn detail(c: &crew_hive::Completion) -> String {
    let served = match c.model.as_str() {
        "" => "another model",
        m => m,
    };
    match c.fell_back.as_str() {
        "" => String::new(),
        skipped => format!("{served} \u{b7} {skipped}"),
    }
}

/// The agents smith has told, this turn, that their chain fell through.
#[derive(Default)]
pub(crate) struct Told(Vec<String>);

impl Told {
    /// Smith's line for `agent`'s reply usage `u` — `fell back to qwen-flash
    /// · qwen-max: <why>` — the first time `agent` falls back this turn;
    /// `None` after that, and for a reply its first model answered.
    pub(crate) fn line(&mut self, agent: &str, u: &Usage) -> Option<PluginEvent> {
        if u.fell_back.is_empty() || self.0.iter().any(|a| a == agent) {
            return None;
        }
        self.0.push(agent.to_string());
        Some(msg("agent smith", format!("fell back to {}", u.fell_back)))
    }
}

#[cfg(test)]
#[path = "fellback_tests.rs"]
mod tests;
