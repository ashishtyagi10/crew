//! `workspace/symbol`: find a symbol by name across the project, with the
//! kind the server knows — what an agent otherwise greps for and gets
//! comments, strings and call sites back with. The reply arrives as
//! `SymbolInformation[]` (a location with a range) or `WorkspaceSymbol[]`
//! (LSP 3.17: the location may be just a `uri`, the range left to
//! `workspaceSymbol/resolve`, which is not worth a round trip here — the
//! file alone still tells an agent where to look).
use std::time::Duration;

use serde_json::{json, Value};

use crate::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// The protocol's `SymbolKind` as a word a person says.
    pub kind: &'static str,
    pub uri: String,
    /// Zero-based, when the server gave a range.
    pub line: Option<u32>,
}

/// `SymbolKind` 1–26 spelled out; anything else (a newer protocol) is "symbol".
pub fn kind_label(code: u64) -> &'static str {
    const KINDS: [&str; 26] = [
        "file",
        "module",
        "namespace",
        "package",
        "class",
        "method",
        "property",
        "field",
        "constructor",
        "enum",
        "interface",
        "function",
        "variable",
        "constant",
        "string",
        "number",
        "boolean",
        "array",
        "object",
        "key",
        "null",
        "enum member",
        "struct",
        "event",
        "operator",
        "type parameter",
    ];
    code.checked_sub(1)
        .and_then(|i| KINDS.get(i as usize))
        .copied()
        .unwrap_or("symbol")
}

impl Symbol {
    fn from_json(v: &Value) -> Option<Self> {
        let loc = v.get("location")?;
        Some(Self {
            name: v.get("name")?.as_str()?.to_string(),
            kind: kind_label(v.get("kind").and_then(Value::as_u64).unwrap_or(0)),
            uri: loc.get("uri")?.as_str()?.to_string(),
            line: loc
                .pointer("/range/start/line")
                .and_then(Value::as_u64)
                .map(|n| n as u32),
        })
    }

    /// Every symbol in a reply of either shape; `null` is none.
    pub fn parse_many(v: &Value) -> Vec<Self> {
        v.as_array()
            .map(|a| a.iter().filter_map(Self::from_json).collect())
            .unwrap_or_default()
    }
}

/// Best matches first: the exact name, then a name that starts with the
/// query (case-insensitive), then the rest — each group in the order the
/// servers gave, since a server's own fuzzy score is the best signal left.
pub fn rank(query: &str, mut list: Vec<Symbol>) -> Vec<Symbol> {
    let q = query.to_lowercase();
    list.sort_by_key(|s| match () {
        _ if s.name == query => 0,
        _ if s.name.to_lowercase().starts_with(&q) => 1,
        _ => 2,
    });
    list
}

impl Client {
    pub fn workspace_symbols(
        &mut self,
        query: &str,
        timeout: Duration,
    ) -> Result<Vec<Symbol>, String> {
        let r = self.request("workspace/symbol", json!({"query": query}), timeout)?;
        Ok(Symbol::parse_many(&r))
    }
}

#[cfg(test)]
#[path = "symbols_tests.rs"]
mod tests;
