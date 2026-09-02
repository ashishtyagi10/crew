//! The arguments a tool needs, checked before anything is filled.
//!
//! `missing argument "lat"` was the whole answer to a call with no arguments,
//! and the model that got it supplied `lat`, called again, and got `missing
//! argument "lon"` — a round per argument, on a tool whose manifest knew all
//! of them. The goal names this failure: an argument typo returns a model
//! apology rather than a validation error. This is the validation error: one
//! message naming every missing argument, what each is for, the optional ones
//! it also takes, and — for a key it does not know — the one it probably meant.
use std::collections::{BTreeMap, BTreeSet};

use super::IntTool;

/// Every `{name}` the tool's path, query and body substitute — what a call
/// MUST supply, whatever the schema says.
pub(crate) fn placeholders(tool: &IntTool) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut text = tool.path.clone();
    for v in tool.query.values() {
        text.push(' ');
        text.push_str(v);
    }
    if let Some(body) = &tool.body {
        text.push(' ');
        text.push_str(&body.to_string());
    }
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|i| open + i) else {
            break;
        };
        let name = &rest[open + 1..close];
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            out.insert(name.to_string());
        }
        rest = &rest[close + 1..];
    }
    out
}

/// The schema's `properties`, as `name → description`, and its `required` names.
fn schema(tool: &IntTool) -> (BTreeMap<String, String>, BTreeSet<String>) {
    let Some(s) = &tool.input_schema else {
        return Default::default();
    };
    let props = s
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|o| {
            o.iter()
                .map(|(k, v)| {
                    let d = v
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    (k.clone(), d)
                })
                .collect()
        })
        .unwrap_or_default();
    let required = s
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (props, required)
}

/// `Ok` when every argument the tool needs is in `given`; otherwise the one
/// message that lets the model fix the call in a single round.
pub(crate) fn check(
    server: &str,
    tool: &IntTool,
    given: &BTreeMap<String, String>,
) -> Result<(), String> {
    let (props, required) = schema(tool);
    let mut needed = placeholders(tool);
    needed.extend(required);
    let missing: Vec<&String> = needed.iter().filter(|n| !given.contains_key(*n)).collect();
    if missing.is_empty() {
        return Ok(());
    }
    let describe = |n: &str| match props.get(n).filter(|d| !d.is_empty()) {
        Some(d) => format!("{n} ({d})"),
        None => n.to_string(),
    };
    let mut msg = format!(
        "{server}:{} is missing {}",
        tool.name,
        missing
            .iter()
            .map(|n| describe(n))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let optional: Vec<String> = props
        .keys()
        .filter(|k| !needed.contains(*k))
        .map(|k| describe(k))
        .collect();
    if !optional.is_empty() {
        msg.push_str(&format!("; it also takes {}", optional.join(", ")));
    }
    // A key the tool does not know, near one it does, is a typo — say which. Argument
    // names are short, so besides the edit distance a name that begins the other
    // (`lat` / `latitude`) counts: the typo rule for tool names would never reach it.
    let known: Vec<&str> = needed
        .iter()
        .chain(props.keys())
        .map(String::as_str)
        .collect();
    for k in given.keys().filter(|k| !known.contains(&k.as_str())) {
        let low = k.to_ascii_lowercase();
        let by_prefix = known
            .iter()
            .find(|n| {
                n.len() >= 3 && low.len() >= 3 && (low.starts_with(*n) || n.starts_with(&low))
            })
            .copied();
        let near = crew_hive::tools::near::nearest(&known, k).first().copied();
        if let Some(meant) = near.or(by_prefix) {
            msg.push_str(&format!(
                "; {k:?} is not an argument \u{2014} did you mean {meant:?}?"
            ));
        }
    }
    if given.is_empty() {
        msg.push_str(". The call had no arguments.");
    } else {
        msg.push('.');
    }
    Err(msg)
}

#[cfg(test)]
#[path = "args_tests.rs"]
mod tests;
