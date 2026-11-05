//! What Claude Code's closing `result` line says beyond the reply: the
//! context the run's last call held, the window it held it in, and the tool
//! calls it refused.
//!
//! `claude -p` cannot ask. A tool the user's settings do not allow is
//! denied, the run goes on, and the line lists each denial in
//! `permission_denials` (captured 2026-11-02 in default mode: a Write and
//! two Bash calls, `{tool_name, tool_use_id, tool_input}`). The reply only
//! says it "needed approval", and a crew pane has no prompt to give it, so
//! the relay names what was refused and where to allow it.
use serde_json::Value;

/// One refused call: the tool, and what it was refused on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Denied {
    pub tool: String,
    pub on: String,
}

/// Most refusals a note names before counting the rest.
const SHOWN: usize = 4;
/// Chars of a refused command the note quotes.
const CMD_MAX: usize = 60;

/// The context a `result` line's usage stands for: the run's LAST call's
/// whole prompt (`iterations`), not its summed, uncached `input_tokens` —
/// "2 in" under every relay reply of a 30,381-token prompt (2026-10-30).
pub(super) fn context_of(usage: &Value) -> u32 {
    let last = usage["iterations"]
        .as_array()
        .and_then(|calls| calls.last())
        .unwrap_or(usage);
    super::served::anthropic_input(last)
}

/// The context window, in tokens, of the model that did the run's work:
/// `modelUsage.<model>.contextWindow` (2.1.x, probed 2026-11-04: 1000000 for
/// claude-opus-5-5). The relay's agent pins no model, so this is the only
/// place its gauge can learn the window. A run that also called a helper
/// model lists both, so the one that read the most prompt is the one asked.
/// 0 when the CLI says none.
pub(super) fn window_of(result: &Value) -> u64 {
    let read = |m: &Value| {
        [
            "inputTokens",
            "cacheReadInputTokens",
            "cacheCreationInputTokens",
        ]
        .iter()
        .map(|k| m[*k].as_u64().unwrap_or(0))
        .sum::<u64>()
    };
    result["modelUsage"]
        .as_object()
        .into_iter()
        .flat_map(|models| models.values())
        .max_by_key(|m| read(m))
        .and_then(|m| m["contextWindow"].as_u64())
        .unwrap_or(0)
}

/// The refusals a `result` line lists, repeats folded.
pub(super) fn denials(result: &Value) -> Vec<Denied> {
    let mut out: Vec<Denied> = Vec::new();
    for d in result["permission_denials"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let input = &d["tool_input"];
        let field = |k: &str| input[k].as_str().map(str::to_string);
        let on = field("file_path")
            .or_else(|| field("command").map(|c| format!("`{}`", clip(&c))))
            .or_else(|| field("notebook_path"))
            .or_else(|| field("url"))
            .or_else(|| field("pattern"))
            .unwrap_or_default();
        let tool = d["tool_name"].as_str().unwrap_or("a tool").to_string();
        let one = Denied { tool, on };
        if !out.contains(&one) {
            out.push(one);
        }
    }
    out
}

/// Smith's verdict row on the run: what was refused, and where to let it
/// through. `None` when nothing was. A `!` row, as the pane's other
/// verdicts (`! not checked`, `! nothing changed`), not a sentence: it rode
/// the reply as prose and read as the agent's own words.
pub fn note(denied: &[Denied]) -> Option<String> {
    if denied.is_empty() {
        return None;
    }
    let named: Vec<String> = denied
        .iter()
        .take(SHOWN)
        .map(|d| match d.on.is_empty() {
            true => d.tool.clone(),
            false => format!("{} {}", d.tool, d.on),
        })
        .collect();
    let more = match denied.len().saturating_sub(SHOWN) {
        0 => String::new(),
        n => format!(" (+{n} more)"),
    };
    Some(format!(
        "! not allowed: {}{more} \u{2014} allow it under permissions.allow in \
         .claude/settings.json",
        named.join(" \u{b7} ")
    ))
}

fn clip(cmd: &str) -> String {
    let one = cmd.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.chars().count() > CMD_MAX {
        true => format!(
            "{}\u{2026}",
            one.chars().take(CMD_MAX - 1).collect::<String>()
        ),
        false => one,
    }
}

#[cfg(test)]
#[path = "clauderesult_tests.rs"]
mod tests;
