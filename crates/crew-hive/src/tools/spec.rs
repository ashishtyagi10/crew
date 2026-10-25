//! A tool as crew spells it.

/// One tool as CREW names it, before any provider gets to see it.
///
/// crew's identity for a tool is `server:tool`, which no provider accepts as a
/// function name — Anthropic and the OpenAI shape both require
/// `[a-zA-Z0-9_-]{1,64}`. Keeping crew's spelling here and doing the encoding
/// in exactly one place ([`ToolCatalog`](super::ToolCatalog)) is what stops
/// `server:tool`, `server__tool` and `server-tool` all existing at once.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    pub server: String,
    pub tool: String,
    pub description: String,
    /// JSON Schema for the arguments. `{"type":"object"}` when the source
    /// declared nothing — never `null`, which providers reject.
    pub input_schema: serde_json::Value,
}

impl ToolSpec {
    /// `server:tool` — crew's spelling, for prompts, events and the ledger.
    pub fn label(&self) -> String {
        format!("{}:{}", self.server, self.tool)
    }
}
