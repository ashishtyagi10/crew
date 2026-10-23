//! `<parameter name="…">` pairs inside `<tool_call>`: a sixth way a model
//! writes a call. qwen3.8-max wrote its edit as
//! `<parameter name="cmd">sys:edit</parameter>` then
//! `<parameter name="args">{"path": …}</parameter>` (live probe, 2026-10-20,
//! a5b-run2); nothing read it, and the turn ended with the edit unrun. The
//! same element carries Anthropic's XML call shape, one parameter per
//! argument under the name, which models trained on its transcripts write.
use serde_json::Value;

/// The parameters that hold the call's name, and the ones that hold its
/// arguments as one JSON object.
const NAMES: [&str; 5] = ["cmd", "name", "tool", "tool_name", "function"];
const ARGS: [&str; 4] = ["args", "arguments", "input", "parameters"];

/// The call opening `s`: name, arguments as written, and how much of `s`
/// they took. The name comes first; then either one JSON arguments
/// parameter, or each argument a parameter of its own.
pub(super) fn call(s: &str) -> Option<(String, String, usize)> {
    let (key, name, mut rest) = element(s)?;
    if !NAMES.contains(&key) {
        return None;
    }
    let mut fields = Vec::new();
    while let Some((key, value, after)) = element(rest.trim_start()) {
        if ARGS.contains(&key) && fields.is_empty() {
            let json = value.trim();
            serde_json::from_str::<Value>(json).ok()?;
            let used = s.len() - after.len();
            return Some((name.trim().to_string(), json.to_string(), used));
        }
        let key = serde_json::to_string(key).ok()?;
        fields.push(format!("{key}: {}", super::param(value)));
        rest = after;
    }
    let args = format!("{{{}}}", fields.join(", "));
    Some((name.trim().to_string(), args, s.len() - rest.len()))
}

/// `<parameter name="KEY">VALUE</parameter>` opening `s`: key, value, rest.
fn element(s: &str) -> Option<(&str, &str, &str)> {
    let rest = s.strip_prefix("<parameter name=\"")?;
    let (key, rest) = rest.split_once("\">")?;
    let (value, rest) = rest.split_once("</parameter>")?;
    Some((key.trim(), value, rest))
}

#[cfg(test)]
#[path = "paramcall_tests.rs"]
mod tests;
