//! Live tool-call lines for the crew pane: every `HiveEvent::ToolCall` an
//! agent makes is one [`ToolLine`] — pending with a spinner and a counting
//! clock, then `✓ label · 120 ms` / `✗ label · 3.2 s` when its `ToolResult`
//! lands. The lines of one agent form a [`ToolBlock`] that sits under the
//! agent's streaming card (or stands as its own thin card) and, once the
//! agent's reply settles, collapses to one summary line above it. This
//! module is the model; the per-line render is `chattoolline`, the block
//! layout and placement `chattoolview`, the click plumbing `chattoolfold`.
use std::collections::HashMap;

use crew_hive::HiveEvent;

use crate::chattoolkind::LineKind;

pub(crate) use crate::chattoolblock::ToolBlock;

/// A finished call's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolDone {
    pub ok: bool,
    pub ms: u64,
    pub text: String,
}

/// One tool call, from the moment it was asked for — or one load
/// (`chattoolkind`), born done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolLine {
    pub kind: LineKind,
    pub label: String,
    /// The arguments, whole — a click opens them (`chattoolargs`).
    pub args: String,
    /// The one argument the line itself shows (`chattoolline::subject`).
    pub args_short: String,
    pub started_ms: u64,
    pub done: Option<ToolDone>,
    /// The arguments and result text are opened under the line (click).
    pub show_text: bool,
}

impl ToolLine {
    /// Whether a click on the line has anything to open: arguments, a
    /// result with text, or a load's detail.
    pub(crate) fn has_text(&self) -> bool {
        !self.args.trim().is_empty()
            || self
                .done
                .as_ref()
                .is_some_and(|d| !d.text.trim().is_empty())
    }
}

/// The pane's tool blocks plus the agent-id → name binding they need.
#[derive(Default)]
pub(crate) struct ToolLines {
    pub blocks: Vec<ToolBlock>,
    pub(crate) names: HashMap<u64, String>,
    /// The agent the last hive event named — the one the broker's next
    /// `Activity { from: "hive" }` is about (it is emitted right after).
    pub(crate) last_agent: Option<u64>,
}

impl ToolLines {
    pub(crate) fn open_block(&mut self, id: u64) -> &mut ToolBlock {
        let open = self
            .blocks
            .iter()
            .rposition(|b| b.agent_id == id && !b.settled);
        let i = open.unwrap_or_else(|| {
            let agent = self
                .names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| format!("agent-{id}"));
            self.blocks.push(ToolBlock {
                agent_id: id,
                agent,
                lines: Vec::new(),
                anchor: None,
                settled: false,
                expanded: false,
            });
            self.blocks.len() - 1
        });
        &mut self.blocks[i]
    }

    /// Fold one hive event in: a call opens a pending line, a result closes
    /// the OLDEST pending line with that label (calls can overlap).
    pub(crate) fn absorb(&mut self, ev: &HiveEvent, now_ms: u64) {
        match ev {
            HiveEvent::AgentSpawned { agent, .. } => self.last_agent = Some(agent.0),
            HiveEvent::ToolCall { agent, label, args } => {
                self.last_agent = Some(agent.0);
                self.open_block(agent.0).lines.push(ToolLine {
                    kind: LineKind::Tool,
                    label: label.clone(),
                    args: args.clone(),
                    args_short: crate::chattoolline::subject(args),
                    started_ms: now_ms,
                    done: None,
                    show_text: false,
                });
            }
            HiveEvent::ToolResult {
                agent,
                label,
                ok,
                text,
                ms,
            } => {
                let b = self.open_block(agent.0);
                let oldest = b
                    .lines
                    .iter_mut()
                    .find(|l| l.done.is_none() && l.label == *label);
                if let Some(l) = oldest {
                    l.done = Some(ToolDone {
                        ok: *ok,
                        ms: *ms,
                        text: text.clone(),
                    });
                }
            }
            HiveEvent::Loaded {
                agent,
                kind,
                name,
                detail,
            } => self.absorb_loaded(agent, kind, name, detail),
            _ => {}
        }
    }

    /// The broker named the agent the last hive event was about.
    pub(crate) fn bind(&mut self, name: &str) {
        let Some(id) = self.last_agent else { return };
        self.names.insert(id, name.to_string());
        for b in self.blocks.iter_mut().filter(|b| b.agent_id == id) {
            b.agent = name.to_string();
        }
    }

    /// `agent`'s reply settled as the card stamped `ts`: its open block
    /// anchors above that card, collapsed. No block of its own: a block of
    /// loads alone is waiting for exactly this reply (`settle_loaded`).
    pub(crate) fn settle(&mut self, agent: &str, ts: &str) {
        let open = self
            .blocks
            .iter_mut()
            .rfind(|b| b.agent == agent && !b.settled);
        match open {
            Some(b) => {
                b.anchor = Some(ts.to_string());
                b.settled = true;
                b.expanded = false;
            }
            None => self.settle_loaded(agent, ts),
        }
    }

    /// The run is over: every pending line fails, every open block closes.
    pub(crate) fn abandon(&mut self, now_ms: u64) {
        for b in self.blocks.iter_mut().filter(|b| !b.settled) {
            for l in b.lines.iter_mut().filter(|l| l.done.is_none()) {
                l.done = Some(ToolDone {
                    ok: false,
                    ms: now_ms.saturating_sub(l.started_ms),
                    text: "no result".into(),
                });
            }
            b.settled = true;
        }
    }

    /// Calls in flight across every block.
    pub(crate) fn pending(&self) -> usize {
        self.blocks.iter().map(ToolBlock::pending).sum()
    }
}

#[cfg(test)]
#[path = "chattool_tests.rs"]
mod tests;
