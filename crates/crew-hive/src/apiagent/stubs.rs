//! How a native task's turns are cut to fit the model's context
//! (`overflow`): which results become stubs, what a stub names, and when only
//! the last round is kept. Pure turn work, tested as such.

use crate::provider::{ToolInvocation, ToolOutcome, Turn};

/// Chars of a call's argument named in its stub: a path is the useful part.
const GIST: usize = 80;

/// Rough size of what a request carries, in bytes: the prompt and system
/// (`fixed`) plus every turn's text, call arguments and results.
pub(super) fn size(fixed: usize, turns: &[Turn]) -> usize {
    fixed
        + turns
            .iter()
            .map(|t| match t {
                Turn::Assistant { text, calls } => {
                    let args: usize = calls.iter().map(|c| c.input.to_string().len()).sum();
                    text.len() + args
                }
                Turn::ToolResults(rs) => rs.iter().map(|r| r.content.len()).sum(),
                Turn::User(s) => s.len(),
            })
            .sum::<usize>()
}

/// Cut `turns` to fit: every result but the last round's stubbed, or, when
/// even that is larger than `fit`, the last round alone after the task.
///
/// `fit` is the largest request this task has had answered. Whether the
/// stubbed turns would still overflow cannot be asked without spending the
/// one retry on it, but a request no larger than one that went through a
/// moment ago fits. `false` when nothing could be cut, and a retry would send
/// the request that was just refused.
pub(super) fn shrink(
    turns: &mut Vec<Turn>,
    fixed: usize,
    fit: usize,
    label: &dyn Fn(&ToolInvocation) -> String,
) -> bool {
    let stubbed = stubbed(turns, label);
    let cut = match size(fixed, &stubbed) <= fit {
        true => stubbed,
        false => {
            let from = turns
                .iter()
                .rposition(|t| matches!(t, Turn::Assistant { .. }))
                .unwrap_or(0);
            turns[from..].to_vec()
        }
    };
    let changed = cut != *turns;
    *turns = cut;
    changed
}

/// `turns` with every result but the last round's as a one-line stub, the
/// calls and what the model wrote kept. A stub stubbed again is the same.
pub(super) fn stubbed(turns: &[Turn], label: &dyn Fn(&ToolInvocation) -> String) -> Vec<Turn> {
    let last = turns
        .iter()
        .rposition(|t| matches!(t, Turn::ToolResults(_)));
    let mut calls: &[ToolInvocation] = &[];
    let mut out = Vec::with_capacity(turns.len());
    for (i, t) in turns.iter().enumerate() {
        match t {
            Turn::Assistant { calls: c, .. } => calls = c,
            Turn::ToolResults(rs) if Some(i) != last => {
                // A result no longer than its stub stays: cutting it would
                // save nothing and lose what it said ("no such file").
                let cut = |r: &ToolOutcome| {
                    let call = calls.iter().find(|c| c.id == r.id);
                    let content = stub(call.map_or(r.name.clone(), label), call);
                    match content.len() < r.content.len() {
                        true => ToolOutcome {
                            content,
                            ..r.clone()
                        },
                        false => r.clone(),
                    }
                };
                out.push(Turn::ToolResults(rs.iter().map(cut).collect()));
                continue;
            }
            _ => {}
        }
        out.push(t.clone());
    }
    out
}

/// `[result of sys:read_file a.rs shortened to fit — call it again if you
/// need it]`: which call it was, so the model can tell what it has lost.
fn stub(label: String, call: Option<&ToolInvocation>) -> String {
    let gist = call.map(|c| gist(&c.input)).unwrap_or_default();
    let what = match gist.is_empty() {
        true => label,
        false => format!("{label} {gist}"),
    };
    format!("[result of {what} shortened to fit \u{2014} call it again if you need it]")
}

/// The argument that says what a call was about: its `path`, else its first
/// string, cut to [`GIST`] chars.
fn gist(input: &serde_json::Value) -> String {
    let first = || input.as_object()?.values().find_map(|v| v.as_str());
    let said = input.get("path").and_then(|v| v.as_str()).or_else(first);
    let said = said
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>();
    let said = said.join(" ");
    match said.chars().count() > GIST {
        true => format!("{}\u{2026}", said.chars().take(GIST).collect::<String>()),
        false => said,
    }
}

#[cfg(test)]
#[path = "stubs_tests.rs"]
mod tests;
