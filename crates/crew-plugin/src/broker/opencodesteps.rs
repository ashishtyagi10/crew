//! Which text in an opencode event stream is the ANSWER. opencode 1.18 runs
//! a question as steps, one model call each, from `step_start` to
//! `step_finish`. A step that finishes with reason `tool-calls` is the model
//! thinking aloud on its way to a tool ("Let me look at…"). The reply is the
//! step that finishes with `stop`. Joining every step's text is how a relay
//! of "which files would I change to add a new sys: tool to agent smith?"
//! came back as 1,500 characters of "Let me understand the context better …
//! Wait, let me re-read" and no answer.
use serde_json::Value;

/// One model call's worth of the stream.
#[derive(Default)]
struct Step {
    texts: Vec<String>,
    /// `step_finish`'s `part.reason`. None when the stream was cut before
    /// the step finished, so a half-written answer still counts as one.
    reason: Option<String>,
}

/// Whether the stream is step-shaped at all. Older opencode, and the shapes
/// `normalize_tests.rs` pins, have no `step_start`; for those, sweeping every
/// `text` key is still the only reading there is.
pub(super) fn has_steps(events: &[Value]) -> bool {
    events.iter().any(|v| kind(v) == Some("step_start"))
}

/// The text of the last step that did not stop to call a tool. When every
/// step with text was narration (the stream ended on a tool call), the
/// latest narration beats an empty reply. None when no step has text, so
/// the caller falls back to error events and then the raw stream.
pub(super) fn final_step_text(events: &[Value]) -> Option<String> {
    let steps = split_steps(events);
    let spoken = || steps.iter().rev().filter(|s| !s.texts.is_empty());
    let step = spoken()
        .find(|s| s.reason.as_deref() != Some("tool-calls"))
        .or_else(|| spoken().next())?;
    Some(step.texts.join("\n\n").trim().to_string())
}

fn split_steps(events: &[Value]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for v in events {
        match kind(v) {
            Some("step_start") => steps.push(Step::default()),
            Some("step_finish") => {
                if let Some(step) = steps.last_mut() {
                    step.reason = part(v)
                        .and_then(|p| p.get("reason"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
            }
            _ => {
                let Some(text) = text_part(v) else { continue };
                if steps.is_empty() {
                    steps.push(Step::default()); // text before any step_start
                }
                if let Some(step) = steps.last_mut() {
                    step.texts.push(text.to_string());
                }
            }
        }
    }
    steps
}

/// The event's own `part.text`, only when `part.type` is `text`. A
/// `reasoning` part carries a `text` too, and a `tool_use` keeps what the
/// tool returned under `state`: reading `crates/crew-theme/src/lib.rs` puts
/// the whole file at `state.metadata.display.text`, which the old sweep of
/// every `text` key pasted into the reply.
fn text_part(v: &Value) -> Option<&str> {
    let part = part(v)?;
    if part.get("type").and_then(Value::as_str) != Some("text") {
        return None;
    }
    let text = part.get("text").and_then(Value::as_str)?;
    (!text.trim().is_empty()).then_some(text)
}

fn part(v: &Value) -> Option<&Value> {
    v.get("part")
}

fn kind(v: &Value) -> Option<&str> {
    v.get("type").and_then(Value::as_str)
}

#[cfg(test)]
#[path = "opencodesteps_tests.rs"]
mod tests;
