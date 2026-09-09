//! The handful of LSP shapes crew reads, parsed by hand from `serde_json`
//! values. Hand-parsed rather than derived because every one of them arrives
//! in more than one spelling — a `definition` is a `Location`, a list of
//! them, a `LocationLink`, or `null` — and a derive that accepts one spelling
//! is a parser that fails on the next server.
use std::path::PathBuf;

use serde_json::Value;

/// A zero-based line and a zero-based UTF-16 column, as the protocol counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Somewhere in some file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

/// One thing a server has to say about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Severity,
    pub message: String,
    /// Who said it — `rustc`, `rust-analyzer`, `pyright` — when the server says.
    pub source: Option<String>,
}

fn u32_of(v: &Value, key: &str) -> Option<u32> {
    v.get(key)?.as_u64().map(|n| n as u32)
}

impl Position {
    pub fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            line: u32_of(v, "line")?,
            character: u32_of(v, "character")?,
        })
    }
}

impl Range {
    pub fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            start: Position::from_json(v.get("start")?)?,
            end: Position::from_json(v.get("end")?)?,
        })
    }
}

impl Location {
    /// A `Location` or a `LocationLink` (whose target is what a person wants).
    pub fn from_json(v: &Value) -> Option<Self> {
        if let Some(uri) = v.get("targetUri").and_then(Value::as_str) {
            let range = v
                .get("targetSelectionRange")
                .or_else(|| v.get("targetRange"))
                .and_then(Range::from_json)?;
            return Some(Self {
                uri: uri.to_string(),
                range,
            });
        }
        Some(Self {
            uri: v.get("uri")?.as_str()?.to_string(),
            range: Range::from_json(v.get("range")?)?,
        })
    }

    /// Every location in a `definition`/`references` result: `null`, one, or
    /// a list — all three are legal replies.
    pub fn parse_many(v: &Value) -> Vec<Self> {
        match v {
            Value::Array(items) => items.iter().filter_map(Self::from_json).collect(),
            Value::Null => Vec::new(),
            one => Self::from_json(one).into_iter().collect(),
        }
    }

    /// The file this points at, when the URI is a `file:` one.
    pub fn path(&self) -> Option<PathBuf> {
        super::uri::to_path(&self.uri)
    }
}

impl Severity {
    /// The protocol's 1–4; an omitted severity is an error, which is what
    /// the spec says a client should assume.
    pub fn from_code(n: Option<u64>) -> Self {
        match n {
            Some(2) => Severity::Warning,
            Some(3) => Severity::Information,
            Some(4) => Severity::Hint,
            _ => Severity::Error,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "info",
            Severity::Hint => "hint",
        }
    }
}

impl Diagnostic {
    pub fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            range: Range::from_json(v.get("range")?)?,
            severity: Severity::from_code(v.get("severity").and_then(Value::as_u64)),
            message: v.get("message")?.as_str()?.to_string(),
            source: v.get("source").and_then(Value::as_str).map(str::to_string),
        })
    }

    /// The `(uri, diagnostics)` of a `textDocument/publishDiagnostics`.
    pub fn parse_publish(params: &Value) -> Option<(String, Vec<Self>)> {
        let uri = params.get("uri")?.as_str()?.to_string();
        let list = params
            .get("diagnostics")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Self::from_json).collect())
            .unwrap_or_default();
        Some((uri, list))
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
