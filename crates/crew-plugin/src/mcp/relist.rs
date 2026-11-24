//! A server's tools listed again when it says they changed.
//!
//! Only where the catalog is read ([`McpHost::tools`]: a turn's start, the
//! planner's capabilities, a `find_tools` search) — never inside a call. The
//! list a turn OFFERS the model is fixed when the turn starts, so a change
//! reaches the model at the next turn (the next hop of a /crew task counts):
//! a list that moved between two rounds of one turn would break the prompt
//! cache's prefix and leave earlier tool calls naming tools it no longer
//! sees. The LOG says what came and went.
//!
//! A listing that fails keeps the list the server had and drops the client:
//! the next read reconnects and tries again, where emptying the cache would
//! have hidden every tool and failed every call until `/reload`.
use super::{McpHost, McpTool};

impl McpHost {
    /// List `server` again when it announced a change (now or before a
    /// dropped connection), and say what changed.
    pub(super) fn relist_if_changed(&mut self, server: &str) {
        if !self.cache.contains_key(server) {
            return;
        }
        let heard = self
            .clients
            .get_mut(server)
            .is_some_and(|c| c.take_tools_changed());
        if heard {
            self.relist.insert(server.to_string());
        }
        if !self.relist.contains(server) {
            return;
        }
        let before = self.cache.remove(server).unwrap_or_default();
        match self.fetch_listing(server) {
            Ok(after) => {
                self.relist.remove(server);
                if let Some(sink) = &self.sink {
                    sink(false, &changed_note(server, &before, &after));
                }
            }
            Err(_) => {
                self.cache.insert(server.to_string(), before);
                self.drop_client(server);
            }
        }
    }

    /// Drop `server`'s connection: the next use reconnects. A tools notice its
    /// client heard is kept for the reconnect, and a drop starts a new
    /// connection generation, so the server's note keys are freed and the
    /// reconnect announces itself (once) too.
    pub(super) fn drop_client(&mut self, server: &str) {
        if let Some(mut c) = self.clients.remove(server) {
            if c.take_tools_changed() {
                self.relist.insert(server.to_string());
            }
        }
        self.noted
            .retain(|k| k != server && !k.starts_with(&format!("{server}:")));
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
