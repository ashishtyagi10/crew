//! A fix request that changed nothing says so.
//!
//! `chatproof` speaks only for a turn that edited files. A turn asked to
//! "fix the failing test" that edited nothing and answered "Fixed — the
//! test passes now" left no mark at all, so a diagnosis read as a repair
//! and the user found out at the next run. A staff engineer reviewing that
//! hand-off checks the diff first: none, so it is a diagnosis. The pane now
//! says that, once, under the answer:
//! `! nothing changed — this answer is a diagnosis, not a fix`.
//!
//! It speaks only when all of it holds, so an honest answer never earns it:
//! the ask was a fix (`fix…`, `repair…`), no edit was heard (`chatproof`),
//! the broker sent no receipt (it measures the tree itself, so an agent
//! whose edits stream no events is still seen in a git project), the turn
//! did not stop or fail, and the answer claims the fix in a sentence that
//! does not hedge it (`would`, `if`, `not`, `already`…).
use crate::chatcard::{is_system_voice, is_tool_card};
use crate::chatlayout::Message;

/// The note under a fix request whose answer claimed a fix nobody made.
pub(crate) const NOTE: &str = "! nothing changed \u{2014} this answer is a diagnosis, not a fix";

/// Words that claim the work is done.
const CLAIMS: &[&str] = &[
    "fixed",
    "resolved",
    "repaired",
    "now passes",
    "now pass ",
    "now works",
    "now compiles",
    "works now",
    "passes now",
    "should now",
    "i've updated",
    "i updated",
    "i've changed",
    "i changed",
    "i've applied",
    "i applied",
    "i've made",
];

/// Words that turn a claim into advice, a condition or a denial.
const HEDGES: &[&str] = &[
    "not ",
    "n't",
    "never",
    "would",
    "could",
    "if ",
    "once ",
    "unless",
    "until",
    "already",
    "you can",
    "you need",
    "you'll",
    "you should",
    "to fix",
    "nothing",
    "no change",
    "?",
];

/// The note for the turn that just ended, given whether `chatproof` heard
/// it edit: `None` unless every condition above holds.
pub(crate) fn note(msgs: &[Message], edited: bool) -> Option<&'static str> {
    if edited {
        return None;
    }
    let at = msgs.iter().rposition(|m| m.sender == "user")?;
    if !asks_a_fix(&msgs[at].text) {
        return None;
    }
    let after = &msgs[at + 1..];
    let smith = |m: &&Message| is_system_voice(&m.sender);
    // A receipt (files changed) or a `✗` ending (stopped, failed) says what
    // happened already, in the broker's own words.
    if after
        .iter()
        .filter(smith)
        .any(|m| crate::chatreceipt::is_receipt(m) || m.text.starts_with('\u{2717}'))
    {
        return None;
    }
    let said = after
        .iter()
        .filter(|m| !is_system_voice(&m.sender) && !is_tool_card(m));
    said.into_iter()
        .any(|m| claims_a_fix(&m.text))
        .then_some(NOTE)
}

/// The ask is a fix: a word that starts `fix` or `repair` (`fix`, `fixing`,
/// not `prefix`).
pub(crate) fn asks_a_fix(ask: &str) -> bool {
    ask.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.starts_with("fix") || w.starts_with("repair"))
}

/// Some sentence of the answer claims the fix outright.
pub(crate) fn claims_a_fix(answer: &str) -> bool {
    let lower = answer.to_lowercase().replace('\u{2019}', "'");
    let claimed = sentences(&lower).any(|s| {
        let s = format!("{s} ");
        CLAIMS.iter().any(|c| s.contains(c)) && !HEDGES.iter().any(|h| s.contains(h))
    });
    claimed
}

/// The answer's sentences, a question mark kept so it hedges its own.
fn sentences(text: &str) -> impl Iterator<Item = &str> {
    text.split_inclusive(['.', '!', '?', '\n', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "chatnofix_tests.rs"]
mod tests;
