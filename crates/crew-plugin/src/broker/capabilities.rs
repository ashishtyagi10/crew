//! What this session can reach, one line per source — the planner's view of
//! the tool surface (`crew_hive::tools::Tools::capabilities`).
//!
//! Coarse by design. The per-task hint already chooses tools within a budget;
//! this is the sentence ABOVE that, so a plan can say "the weather task is
//! reachable through `weather`" without seeing forty tool rows.
use crate::mcp::McpTool;

use super::integration::Integration;

/// Tool names shown per MCP server before `+N more`.
const NAMES: usize = 6;

/// One line per integration (its manifest's description, or its tools when it
/// has none), then one per MCP server (its tools, the first few). `sys` — the
/// shell, the clock, the finder — is left out: it is not a place, and every
/// planner run has it.
pub(super) fn lines(ints: &[Integration], mcp: &[McpTool]) -> Vec<String> {
    let mut out: Vec<String> = ints
        .iter()
        .map(|i| {
            let what = if i.description.trim().is_empty() {
                names(i.tools.iter().map(|t| t.name.as_str()))
            } else {
                i.description.trim().to_string()
            };
            format!("{}: {what}", i.name)
        })
        .collect();
    let mut servers: Vec<&str> = mcp.iter().map(|t| t.server.as_str()).collect();
    servers.sort_unstable();
    servers.dedup();
    for s in servers {
        let held = names(
            mcp.iter()
                .filter(|t| t.server == s)
                .map(|t| t.name.as_str()),
        );
        out.push(format!("{s} (an MCP server): {held}"));
    }
    out
}

/// `a, b, c, +4 more` — enough to say what kind of thing a source is.
fn names<'a>(it: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<&str> = it.collect();
    let shown: Vec<&str> = all.iter().take(NAMES).copied().collect();
    let mut s = shown.join(", ");
    if all.len() > NAMES {
        s.push_str(&format!(", +{} more", all.len() - NAMES));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(server: &str, name: &str) -> McpTool {
        McpTool {
            server: server.into(),
            name: name.into(),
            description: String::new(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    /// An integration is its description; a server is what it holds, a few names deep.
    #[test]
    fn one_line_per_source_and_a_server_names_a_few_of_its_tools() {
        let mut mcp: Vec<McpTool> = (0..8).map(|i| tool("gws", &format!("t{i}"))).collect();
        mcp.push(tool("fs", "read"));
        let lines = lines(&[], &mcp);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0], "fs (an MCP server): read");
        assert_eq!(
            lines[1],
            "gws (an MCP server): t0, t1, t2, t3, t4, t5, +2 more"
        );
    }

    #[test]
    fn nothing_reachable_is_no_lines() {
        assert!(lines(&[], &[]).is_empty());
    }
}
