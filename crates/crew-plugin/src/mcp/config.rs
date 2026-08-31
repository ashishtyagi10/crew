//! `mcp.json` — which MCP servers exist and how to launch them. The schema is
//! the `mcpServers` map other coding tools already use, so a config can be
//! copied across verbatim. Merged from `~/.config/crew/mcp.json` (user) and
//! `./.crew/mcp.json` (project; wins on a name collision).
use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// How to launch one stdio MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment for the server process (merged over the broker's).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct McpFile {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, ServerConfig>,
}

/// Parse one `mcp.json`; unreadable or malformed content is an empty map.
pub(crate) fn parse(text: &str) -> BTreeMap<String, ServerConfig> {
    serde_json::from_str::<McpFile>(text)
        .map(|f| f.mcp_servers)
        .unwrap_or_default()
}

fn load_file(path: &Path) -> BTreeMap<String, ServerConfig> {
    std::fs::read_to_string(path)
        .map(|t| parse(&t))
        .unwrap_or_default()
}

/// The merged server map: user config first, project entries on top.
pub(crate) fn load() -> BTreeMap<String, ServerConfig> {
    let mut all = dirs::config_dir()
        .map(|d| load_file(&d.join("crew").join("mcp.json")))
        .unwrap_or_default();
    all.extend(load_file(Path::new(".crew/mcp.json")));
    all
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
