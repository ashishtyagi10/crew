//! The recalled block: what the graph found, rendered for the front of a task.
//!
//! Shaped like the thread's block (`thread::context`) on purpose — same
//! voice, same clip marker, same "newest first, freshest survives a tight
//! budget" rule — because from the model's side they are one paragraph of
//! context with two sources: this session (the thread) and every session
//! before it (the graph).
use crate::broker::thread::clip_chars;

use super::graph::Graph;
use super::query;

/// Chars the block may take in front of a task. Half the thread's cap: a
/// recollection is a prompt, not the record.
pub(crate) const RECALL_CAP: usize = 1024;
/// Turns recalled at most, however much budget is left.
const TURNS_MAX: usize = 4;
/// Files named on the trailing line.
const FILES_MAX: usize = 4;
/// Pages named on the trailing line. Fewer than the files, because a URL is
/// four times the width of a path and the block is paying by the char.
const PAGES_MAX: usize = 2;
const HEAD: &str = "From earlier work in this project:";

/// What a recall came to: the block itself and the counts the pane says out
/// loud (`context: … recalled 2 turns (up to 3w ago)`). One value, because a
/// line that reports a recall the run did not actually carry is worse than
/// no line — the counts are taken from the rendered block, not from a second
/// query that might rank differently.
pub(crate) struct Recalled {
    pub text: String,
    pub turns: usize,
    /// Epoch-ms of the OLDEST turn quoted — "from 3w ago" is about reach.
    ///
    /// Only the turns are counted. The files line is part of the block and
    /// not part of the claim: the pane says how far back the MEMORY reached,
    /// and a count of paths would read like a promise the run touched them.
    pub oldest_ms: u64,
}

/// The block, or `None` when the graph has nothing to say about `text` — in
/// which case the task passes through byte-identical, which is what the test
/// for a cold start asserts.
pub(crate) fn block(
    g: &Graph,
    text: &str,
    skip: &[String],
    now_ms: u64,
    budget: usize,
) -> Option<Recalled> {
    let mut left = budget.checked_sub(HEAD.len() + 8)?;
    let mut lines: Vec<String> = Vec::new();
    let mut oldest_ms = u64::MAX;
    let mut quoted = 0usize;
    let hits = query::turns(g, text, skip, TURNS_MAX);
    for h in &hits {
        let Some(n) = g.node(h.id) else { continue };
        let share = left / TURNS_MAX.max(1);
        let body = flatten(&n.text);
        let room = share.max(64).min(left);
        if room < 48 {
            break;
        }
        let line = format!(
            "- {} — {}",
            ago(n.last_ms, now_ms),
            clip_chars(&body, room - 24)
        );
        left = left.saturating_sub(line.chars().count() + 1);
        oldest_ms = oldest_ms.min(n.last_ms);
        quoted += 1;
        lines.push(line);
    }
    let files = query::files(g, text, FILES_MAX);
    if !files.is_empty() {
        let line = format!("- files that came up: {}", files.join(", "));
        if line.chars().count() < left {
            // Charged against the budget, which it did not have to be while
            // it was the last line in the block. It is not any more.
            left -= line.chars().count();
            lines.push(line);
        }
    }
    // After the files, because a path is where work happened and a URL is
    // only where an answer came from — and last in means first cut when the
    // budget is tight.
    let pages = query::pages(g, text, PAGES_MAX);
    if !pages.is_empty() {
        let line = format!("- pages read: {}", pages.join(", "));
        if line.chars().count() < left {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(Recalled {
        text: format!("{HEAD}\n{}", lines.join("\n")),
        turns: quoted,
        oldest_ms: oldest_ms.min(now_ms),
    })
}

/// A turn is stored over two lines; in the block it is one.
fn flatten(s: &str) -> String {
    s.replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// How long ago, in the coarsest unit that is still true. A clock the user
/// never set, so never a date: "3 days ago" needs no timezone to be right.
pub(crate) fn ago(then_ms: u64, now_ms: u64) -> String {
    let secs = now_ms.saturating_sub(then_ms) / 1000;
    match secs {
        0..=90 => "just now".to_string(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86_400 => format!("{}h ago", s / 3600),
        s if s < 86_400 * 14 => format!("{}d ago", s / 86_400),
        s => format!("{}w ago", s / (86_400 * 7)),
    }
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
