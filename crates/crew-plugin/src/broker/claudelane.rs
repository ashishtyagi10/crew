//! What one line of a live `claude -p` run tells the pane — reply text, the
//! model's reasoning, and each of Claude Code's tool calls with its outcome
//! and how long it took — and what the folded run comes to at its end. The
//! relay's read loop (`claudeagent.rs`) reads the lines; this hands the pane
//! everything it shows while the run is live.
use std::collections::HashMap;
use std::time::Instant;

use crew_hive::claudestream::{Fold, StreamEvent};
use crew_hive::{AgentId, HiveEvent};

use super::adapter::{HopStream, Usage};
use super::run::explain_failure;

/// The run's open tool calls and the reply's size so far.
pub(super) struct Lane {
    agent: AgentId,
    open: HashMap<String, (String, Instant)>,
    chars: u64,
}

impl Lane {
    pub(super) fn new() -> Self {
        Self {
            agent: AgentId::minted("claude"),
            open: HashMap::new(),
            chars: 0,
        }
    }

    /// Hand `ev` to the pane through `stream`.
    pub(super) fn forward(&mut self, ev: Option<&StreamEvent>, stream: &HopStream) {
        match ev {
            Some(StreamEvent::Text(t)) => {
                (stream.on_text)(t);
                self.chars += t.chars().count() as u64;
                (stream.on_tokens)(self.chars / 4);
            }
            Some(StreamEvent::Thought(t)) => (stream.on_thought)(t),
            Some(StreamEvent::ToolUse { id, name, input }) => {
                self.open.insert(id.clone(), (name.clone(), Instant::now()));
                (stream.on_tool)(HiveEvent::ToolCall {
                    agent: self.agent.clone(),
                    label: name.clone(),
                    args: input.clone(),
                });
            }
            Some(StreamEvent::ToolResult { id, ok, text }) => {
                let (label, t0) = self
                    .open
                    .remove(id)
                    .unwrap_or_else(|| ("tool".into(), Instant::now()));
                (stream.on_tool)(HiveEvent::ToolResult {
                    agent: self.agent.clone(),
                    label,
                    ok: *ok,
                    text: text.clone(),
                    ms: t0.elapsed().as_millis() as u64,
                });
            }
            _ => {}
        }
    }
}

/// The reply a folded run is: the CLI's own refusal reads as the error it
/// printed; an empty run is explained from stderr (`run::explain_failure`).
pub(super) fn finish(fold: Fold, program: &str, stderr: &str) -> Result<(String, Usage), String> {
    if let Some(r) = fold.result.as_ref() {
        if r.is_error || (!r.subtype.is_empty() && r.subtype != "success") {
            let text = r.text.trim();
            return Err(if text.is_empty() {
                format!("{program} exited with {}", r.subtype)
            } else {
                format!("{program}: {text}")
            });
        }
    }
    let text = fold.reply();
    if text.is_empty() {
        return Err(explain_failure(program, stderr));
    }
    let r = fold.result.unwrap_or_default();
    Ok((
        text,
        Usage {
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            // No cost: the plan covers it, and a list-price figure would
            // show a bill the user never gets.
            ..Default::default()
        },
    ))
}
