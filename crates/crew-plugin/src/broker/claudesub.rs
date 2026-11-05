//! What a Claude Code subagent did, folded under the lead's `Agent`/`Task`
//! call that spawned it. The subagent's own calls never reach the pane as
//! lines of their own: a wall of someone else's Reads buries the lead's
//! work, and a subagent's `TodoWrite` would replace the lead's drawn plan
//! (`hoppane` matches a checklist by label alone). Instead they are counted
//! here, and the spawning call's result opens with the count — the first
//! line of a result is what the pane's done line previews, so the folded
//! line reads `✓ Agent explore the renderer · 41s  14 calls · 6 Read …`.
use std::collections::HashMap;

use crew_hive::claudestream::StreamEvent;

/// Tool kinds named in the tally; the rest are in the total only.
const KINDS: usize = 3;

/// One subagent's calls, by tool, in the order each tool first appeared.
#[derive(Default)]
struct Tally {
    kinds: Vec<(String, u32)>,
    failed: u32,
}

/// Every subagent of the run, keyed by the lead's call id that spawned it.
#[derive(Default)]
pub(super) struct Subagents {
    by_parent: HashMap<String, Tally>,
}

impl Subagents {
    /// Count one line a subagent wrote under `parent`.
    pub(super) fn absorb(&mut self, parent: &str, ev: &StreamEvent) {
        let t = self.by_parent.entry(parent.to_string()).or_default();
        match ev {
            StreamEvent::ToolUse { name, .. } => match t.kinds.iter_mut().find(|k| k.0 == *name) {
                Some(k) => k.1 += 1,
                None => t.kinds.push((name.clone(), 1)),
            },
            StreamEvent::ToolResult { ok: false, .. } => t.failed += 1,
            _ => {}
        }
    }

    /// The result text of the lead's call `id`, led by its subagent's tally
    /// when it made any calls. A call that spawned none (or a backgrounded
    /// one, whose result lands before its subagent starts) is left as is.
    pub(super) fn lead(&mut self, id: &str, text: &str) -> String {
        match self.by_parent.remove(id).and_then(|t| summary(&t)) {
            Some(s) if text.trim().is_empty() => s,
            Some(s) => format!("{s}\n{text}"),
            None => text.to_string(),
        }
    }
}

/// `9 calls · 3 Read · 2 Grep · 2 Edit · 1 failed`: the busiest kinds
/// first (ties keep first-seen order), so the line says what the work was.
fn summary(t: &Tally) -> Option<String> {
    let total: u32 = t.kinds.iter().map(|k| k.1).sum();
    if total == 0 {
        return None;
    }
    let mut kinds = t.kinds.clone();
    kinds.sort_by_key(|k| std::cmp::Reverse(k.1));
    let noun = if total == 1 { "call" } else { "calls" };
    let mut parts = vec![format!("{total} {noun}")];
    parts.extend(kinds.iter().take(KINDS).map(|(n, c)| format!("{c} {n}")));
    if t.failed > 0 {
        parts.push(format!("{} failed", t.failed));
    }
    Some(parts.join(" \u{00b7} "))
}

#[cfg(test)]
#[path = "claudesub_tests.rs"]
mod tests;
