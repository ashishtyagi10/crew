//! What one line of a live `claude -p` run tells the pane: reply text, the
//! model's reasoning, and each of Claude Code's tool calls with its outcome
//! and how long it took. The relay's read loop (`claudeagent.rs`) folds the
//! reply; this hands the pane everything it shows while the run is live.
use std::collections::HashMap;
use std::time::Instant;

use crew_hive::claudestream::StreamEvent;
use crew_hive::{AgentId, HiveEvent};

use super::adapter::HopStream;

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
