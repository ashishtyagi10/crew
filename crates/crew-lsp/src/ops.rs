//! The typed questions crew asks a server, over [`Client::request`]. Each
//! takes protocol positions — zero-based line, zero-based UTF-16 column —
//! and leaves the one-based arithmetic to the caller, who knows what a
//! person typed.
use std::time::Duration;

use serde_json::{json, Value};

use crate::types::Location;
use crate::Client;

fn at(uri: &str, line: u32, character: u32) -> Value {
    json!({
        "textDocument": {"uri": uri},
        "position": {"line": line, "character": character},
    })
}

/// A hover's `contents` as one string: `MarkupContent`, a `MarkedString`,
/// or a list of either — every spelling flattened to its text, code blocks
/// re-fenced with their language so the result still reads as markdown.
pub fn flatten_hover(contents: &Value) -> String {
    match contents {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(flatten_hover)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        Value::Object(o) => match (o.get("value").and_then(Value::as_str), o.get("language")) {
            (Some(v), Some(Value::String(lang))) => format!("```{lang}\n{v}\n```"),
            (Some(v), _) => v.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

impl Client {
    /// What the server knows about the symbol at a position, as markdown;
    /// `None` when it has nothing to say.
    pub fn hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> Result<Option<String>, String> {
        let r = self.request("textDocument/hover", at(uri, line, character), timeout)?;
        let text = r.get("contents").map(flatten_hover).unwrap_or_default();
        Ok((!text.trim().is_empty()).then_some(text))
    }

    pub fn definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> Result<Vec<Location>, String> {
        let r = self.request("textDocument/definition", at(uri, line, character), timeout)?;
        Ok(Location::parse_many(&r))
    }

    /// Every use of the symbol, its declaration included.
    pub fn references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> Result<Vec<Location>, String> {
        let mut params = at(uri, line, character);
        params["context"] = json!({"includeDeclaration": true});
        let r = self.request("textDocument/references", params, timeout)?;
        Ok(Location::parse_many(&r))
    }
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
