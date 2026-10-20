//! Two more ways a model writes a call inside `<tool_call>`.
//!
//! Asked for `@tool`, qwen3.8-max wrote every call of two live runs
//! (2026-10-20) in one of these, so none ran: the answer was the raw block,
//! and the judge found "no rename was performed".
//!
//! - The name and the arguments each in a tag of their own:
//!   `<tool_name>sys:read_file</tool_name>\n<arguments>{"path": …}</arguments>`,
//!   with `<function_name>` as often as `<tool_name>`, or the arguments as
//!   GLM's `<arg_key>path</arg_key><arg_value>a.rs</arg_value>` pairs.
//! - The name bare, then its JSON: `sys:edit {"path": …}`.
//!
//! Each says what an `@tool` line says, so `untag` writes one.
use serde_json::Value;

const NAMES: [&str; 3] = ["tool_name", "function_name", "name"];
const ARGS: [&str; 4] = ["arguments", "parameters", "input", "args"];

/// The call opening `s`: name, arguments as written, and how much of `s`
/// they took.
pub(super) fn call(s: &str) -> Option<(String, String, usize)> {
    tagged(s).or_else(|| bare(s))
}

fn tagged(s: &str) -> Option<(String, String, usize)> {
    let (name, rest) = NAMES.iter().find_map(|t| element(s, t))?;
    let rest = rest.trim_start();
    let (args, tail) = wrapped(rest).or_else(|| pairs(rest))?;
    Some((name.trim().to_string(), args, s.len() - tail.len()))
}

/// `<arguments>{…}</arguments>` (or a sibling tag) opening `rest`: the JSON as
/// written, and what follows.
fn wrapped(rest: &str) -> Option<(String, &str)> {
    let (tag, body) = ARGS
        .iter()
        .find_map(|t| rest.strip_prefix(&format!("<{t}>")).map(|b| (t, b)))?;
    let lead = body.len() - body.trim_start().len();
    let n = json_len(&body[lead..])?;
    let mut tail = &body[lead + n..];
    if let Some(t) = tail.trim_start().strip_prefix(&format!("</{tag}>")) {
        tail = t;
    }
    Some((body[lead..lead + n].to_string(), tail))
}

/// `<arg_key>k</arg_key><arg_value>v</arg_value>` pairs opening `rest`, as a
/// JSON object in the order written (values read as `tagged::param` reads
/// Qwen3-Coder's), and what follows.
fn pairs(mut rest: &str) -> Option<(String, &str)> {
    let mut fields = Vec::new();
    while let Some((key, after)) = element(rest.trim_start(), "arg_key") {
        let (value, after) = element(after.trim_start(), "arg_value")?;
        let key = serde_json::to_string(key.trim()).ok()?;
        fields.push(format!("{key}: {}", super::param(value)));
        rest = after;
    }
    (!fields.is_empty()).then(|| (format!("{{{}}}", fields.join(", ")), rest))
}

/// `<tag>inner</tag>` at the start of `s`: the inner text and what follows.
fn element<'a>(s: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let inner = s.strip_prefix(&format!("<{tag}>"))?;
    let close = format!("</{tag}>");
    let end = inner.find(&close)?;
    Some((&inner[..end], &inner[end + close.len()..]))
}

fn bare(s: &str) -> Option<(String, String, usize)> {
    let end = s.find(char::is_whitespace)?;
    let name = &s[..end];
    let word = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    let (server, tool) = name.split_once(':')?;
    if !word(server) || !word(tool) {
        return None;
    }
    let rest = &s[end..];
    let lead = rest.len() - rest.trim_start().len();
    let n = json_len(&rest[lead..])?;
    let args = rest[lead..lead + n].to_string();
    Some((name.to_string(), args, end + lead + n))
}

/// The length of the JSON object `s` opens with, when it closes.
fn json_len(s: &str) -> Option<usize> {
    if !s.starts_with('{') {
        return None;
    }
    let mut values = serde_json::Deserializer::from_str(s).into_iter::<Value>();
    values.next()?.ok()?;
    Some(values.byte_offset())
}

#[cfg(test)]
#[path = "namedcall_tests.rs"]
mod tests;
