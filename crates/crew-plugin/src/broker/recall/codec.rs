//! One record per line, JSON, append-only — the whole on-disk format.
//!
//! Edges name their endpoints by `kind:key` rather than by index. An index
//! would be shorter and would also make the file order-dependent: replay a
//! log whose first lines were lost (a truncated write, a hand edit) and every
//! edge would point at the wrong node, silently. Keys degrade the honest way
//! — an edge whose endpoint is missing is dropped on load.
use serde_json::{json, Value};

use super::node::{Edge, Kind, Node, Rel};

/// A decoded line.
pub(crate) enum Rec {
    Node(Node),
    /// `(from, to, rel, weight)` with endpoints as `kind:key` pairs.
    Edge((Kind, String), (Kind, String), Rel, u32),
}

/// `kind:key`, the form an edge endpoint is written as.
pub(crate) fn addr(kind: Kind, key: &str) -> String {
    format!("{}:{key}", kind.tag())
}

fn split_addr(s: &str) -> Option<(Kind, String)> {
    let (k, key) = s.split_once(':')?;
    Some((Kind::parse(k)?, key.to_owned()))
}

pub(crate) fn node_line(n: &Node) -> String {
    json!({
        "t": "n",
        "k": n.kind.tag(),
        "key": n.key,
        "x": n.text,
        "h": n.hits,
        "ms": n.last_ms,
    })
    .to_string()
}

pub(crate) fn edge_line(e: &Edge, from: &Node, to: &Node) -> String {
    json!({
        "t": "e",
        "f": addr(from.kind, &from.key),
        "to": addr(to.kind, &to.key),
        "r": e.rel.tag(),
        "w": e.weight,
    })
    .to_string()
}

/// Decode one line. A line that is not JSON, or is JSON of the wrong shape,
/// is `None`: a corrupt tail loses its own records and nothing else, which is
/// the failure mode an append-only log is chosen for.
pub(crate) fn decode(line: &str) -> Option<Rec> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    match v.get("t")?.as_str()? {
        "n" => Some(Rec::Node(Node {
            kind: Kind::parse(v.get("k")?.as_str()?)?,
            key: v.get("key")?.as_str()?.to_owned(),
            text: v.get("x").and_then(Value::as_str).unwrap_or("").to_owned(),
            hits: v.get("h").and_then(Value::as_u64).unwrap_or(1) as u32,
            last_ms: v.get("ms").and_then(Value::as_u64).unwrap_or(0),
        })),
        "e" => Some(Rec::Edge(
            split_addr(v.get("f")?.as_str()?)?,
            split_addr(v.get("to")?.as_str()?)?,
            Rel::parse(v.get("r")?.as_str()?)?,
            v.get("w").and_then(Value::as_u64).unwrap_or(1) as u32,
        )),
        _ => None,
    }
}

#[cfg(test)]
#[path = "codec_tests.rs"]
mod tests;
