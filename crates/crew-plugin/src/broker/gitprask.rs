//! Whether a message asks for a pull request to be opened.
//!
//! Read, not classified, like the other human gates: a verb that opens one
//! and the words for it, in a short message, with nothing asking for a
//! review, a fix or a merge. "review my PR" is a review and stays one
//! (`reviewask`, `shapeguard`); a long task that mentions opening a PR at its
//! end is a task for an agent. A miss costs what it did before; a hit only
//! DRAFTS — opening takes the user's own "apply".
use crate::broker::intent::shapeguard::says_any;

/// Asking for one to be opened.
const OPEN: &[&str] = &[
    "open a pr",
    "open the pr",
    "open pr",
    "open a pull request",
    "make a pr",
    "make a pull request",
    "create a pr",
    "create a pull request",
    "raise a pr",
    "submit a pr",
    "send this as a pr",
    "send it as a pr",
    "send this as a pull request",
    "send it as a pull request",
    "push a pr",
];

/// Anything that makes it a different request.
const NOT: &[&str] = &[
    "review", "fix", "merge", "close", "comment", "comments", "address", "resolve", "check",
    "status", "list", "why", "how", "what",
];

/// A message longer than this is a task, not the request.
const MAX_WORDS: usize = 12;

/// Whether `task` asks for a pull request to be drafted and opened.
pub(crate) fn asks_pr(task: &str) -> bool {
    task.split_whitespace().count() <= MAX_WORDS
        && says_any(task, OPEN)
        && !says_any(task, NOT)
        && !task.contains('#')
}

#[cfg(test)]
#[path = "gitprask_tests.rs"]
mod tests;
