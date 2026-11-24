//! A server's tools listed again when it says they changed.
//!
//! The tool cache ([`McpHost::fetch`]) is read at every task's start, so a
//! change lands on the NEXT task, never mid-loop: a tool list that moved
//! between two rounds of one turn would break the prompt cache's prefix and
//! leave the model's earlier tool calls naming tools it can no longer see.
//! The LOG says what changed, so a tool that appears or vanishes is not a
//! mystery.
use super::{McpHost, McpTool};

impl McpHost {
    /// Drop `server`'s cached tools when it announced a change, and say so.
    /// The next [`McpHost::fetch`] lists them again.
    pub(super) fn relist_if_changed(&mut self, server: &str) {
        if !self.cache.contains_key(server) {
            return;
        }
        let changed = self
            .clients
            .get_mut(server)
            .is_some_and(|c| c.take_tools_changed());
        if !changed {
            return;
        }
        let before = self.cache.remove(server).unwrap_or_default();
        let Ok(after) = self.fetch_listing(server) else {
            return;
        };
        if let Some(sink) = &self.sink {
            sink(false, &changed_note(server, &before, &after));
        }
    }

    /// `tools/list` on `server`, cached: the listing alone, which a first
    /// fetch announces as a connect and a relist as a change.
    pub(super) fn fetch_listing(&mut self, server: &str) -> Result<Vec<McpTool>, String> {
        let list = match self.client(server)?.tools() {
            Ok(list) => list,
            Err(e) => {
                // Connected but the tool listing failed — worth its own note
                // (the connect keys are already spent by `client()`).
                self.note_once(
                    format!("{server}:fail"),
                    true,
                    &format!("mcp {server}: {e}"),
                );
                return Err(e);
            }
        };
        let tools: Vec<McpTool> = list
            .into_iter()
            .map(|(name, description, input_schema)| McpTool {
                server: server.to_string(),
                name,
                description,
                input_schema,
            })
            .collect();
        self.cache.insert(server.to_string(), tools.clone());
        Ok(tools)
    }
}

/// `mcp github tools changed · +create_pr −old_tool · 13 tools`: what came,
/// what went, and how many there are now.
pub(crate) fn changed_note(server: &str, before: &[McpTool], after: &[McpTool]) -> String {
    let had = |list: &[McpTool], n: &str| list.iter().any(|t| t.name == n);
    let mut delta: Vec<String> = Vec::new();
    delta.extend(
        after
            .iter()
            .filter(|t| !had(before, &t.name))
            .map(|t| format!("+{}", t.name)),
    );
    delta.extend(
        before
            .iter()
            .filter(|t| !had(after, &t.name))
            .map(|t| format!("\u{2212}{}", t.name)),
    );
    let tools = crate::broker::wording::count(after.len(), "tool");
    match delta.is_empty() {
        true => format!("mcp {server} tools changed \u{b7} {tools}"),
        false => format!(
            "mcp {server} tools changed \u{b7} {} \u{b7} {tools}",
            delta.join(" ")
        ),
    }
}

#[cfg(test)]
#[path = "relist_tests.rs"]
mod tests;
