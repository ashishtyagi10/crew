//! Breakage the graph has seen before.
//!
//! The check (`selfcheck`) already writes every verdict into the graph, so by
//! the time a build breaks the answer to "is this new?" is on disk and nobody
//! reads it. That is the difference between a tool that reports and one that
//! remembers: the second time a project fails the same way, the useful thing
//! to say is not the error again — it is that this is the second time, when
//! the first was, and which files were in the failure then.
//!
//! Two failures are "the same" when their [`signature`]s match: the head of
//! the output with every number taken out, because a line number moves with
//! each edit while the error does not. It is deliberately blunt — a false
//! match costs one misleading sentence, a missed one costs nothing at all.
use super::graph::Graph;
use super::node::{Kind, NodeId};
use super::query::{answered_of, asked_of};

/// Real lines of a failure kept as its stored digest.
const DIGEST_LINES: usize = 2;
/// Chars of a normalised failure compared. Past this, two failures that open
/// identically are the same failure for our purposes.
const SIG_CHARS: usize = 90;
/// Files named from the earlier failure. Two is a hint; the whole list is a
/// catalogue nobody reads.
pub(crate) const PRIOR_FILES: usize = 2;
/// How the check's verdict opens when it failed — the half of a stored turn
/// this module reads.
const FAILED: &str = "failed: ";

/// An earlier failure of the same check, in the same way.
pub(crate) struct Prior {
    /// How many times it has failed this way BEFORE now.
    pub times: usize,
    /// When the most recent of those was.
    pub last_ms: u64,
    /// Paths that earlier failure named.
    pub files: Vec<String>,
}

/// The failure boiled down to what is worth storing and comparing: the first
/// real lines of the output, with the shell's own `exit N` line dropped —
/// that line carries the exit code and nothing about the cause.
pub(crate) fn digest(output: &str) -> String {
    output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("exit "))
        .take(DIGEST_LINES)
        .collect::<Vec<_>>()
        .join(" ")
}

/// What makes two failures the same one. Numbers go (a line number moves
/// with every edit above it), case goes, runs of space collapse.
pub(crate) fn signature(failure: &str) -> String {
    let mut out = String::new();
    let mut space = true;
    for c in failure
        .trim()
        .strip_prefix(FAILED)
        .unwrap_or(failure)
        .chars()
    {
        let c = c.to_ascii_lowercase();
        match c {
            _ if c.is_ascii_digit() => {}
            _ if c.is_whitespace() => {
                if !space {
                    out.push(' ');
                    space = true;
                }
            }
            _ => {
                out.push(c);
                space = false;
            }
        }
        if out.chars().count() >= SIG_CHARS {
            break;
        }
    }
    out.trim().to_string()
}

/// Has `cmd` failed with this output before? `None` the first time, and for
/// a failure whose signature the graph has never held.
pub(crate) fn prior(g: &Graph, cmd: &str, output: &str, files_max: usize) -> Option<Prior> {
    let sig = signature(&digest(output));
    if sig.is_empty() {
        return None;
    }
    let asked = format!("check: {}", cmd.trim());
    let mut hits: Vec<(NodeId, u64)> = g
        .nodes()
        .filter(|(_, n)| n.kind == Kind::Turn && asked_of(&n.text) == asked)
        .filter(|(_, n)| {
            let said = answered_of(&n.text);
            said.starts_with(FAILED) && signature(said) == sig
        })
        .map(|(id, n)| (id, n.last_ms))
        .collect();
    hits.sort_unstable_by_key(|(_, ms)| std::cmp::Reverse(*ms));
    let &(newest, last_ms) = hits.first()?;
    Some(Prior {
        times: hits.len(),
        last_ms,
        files: files_of(g, newest, files_max),
    })
}

/// The paths one remembered turn named, as the graph spells them.
fn files_of(g: &Graph, turn: NodeId, max: usize) -> Vec<String> {
    g.neighbors(turn)
        .filter_map(|(id, _)| {
            let n = g.node(id)?;
            (n.kind == Kind::File).then(|| n.text.clone())
        })
        .take(max)
        .collect()
}

/// What the pane is told, and what the repair pass is handed. Says the count
/// including this one — "the 3rd time" is the fact a person acts on.
pub(crate) fn sentence(p: &Prior, ago: &str) -> String {
    let mut s = format!(
        "seen before: this check failed the same way {ago} \u{2014} the {} time",
        nth(p.times + 1)
    );
    if !p.files.is_empty() {
        s.push_str(&format!(", last in {}", p.files.join(", ")));
    }
    s
}

/// `2nd`, `3rd`, `11th` — the teens are the exception every naive version of
/// this gets wrong.
fn nth(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
#[path = "repeats_tests.rs"]
mod tests;
