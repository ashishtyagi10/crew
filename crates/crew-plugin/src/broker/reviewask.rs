//! "review my diff" reaches the review, whoever leads the turn.
//!
//! Measured live (2026-11-04, loop 39 S8): with Claude Code signed in, a
//! plain message never met the router. It went straight to `@claude` as the
//! turn's lead (`auth::starter`), so "review my diff" became an open task:
//! Claude handed it to opencode ("review, second opinion"), and opencode
//! edited the callers it should have reported. The review capability was
//! only reachable through a classifier that a subscription user never runs.
//!
//! So the ask is read, not classified, like the other human gates
//! (`intent::gate`): a review word and a word naming the working changes,
//! and no word asking for an edit. Narrow on purpose. A miss costs what it
//! did before (an agent answers); a false hit would answer a task with a
//! review nobody asked for.
use crate::broker::intent::shapeguard::says_any;

/// Asking for a review: the verb, or the noun with its verb in it.
const REVIEW: &[&str] = &["review", "look over", "go over", "check over", "critique"];

/// The working changes, which is what the review path reads.
const CHANGES: &[&str] = &[
    "diff",
    "changes",
    "change",
    "patch",
    "staged",
    "uncommitted",
    "edits",
];

/// A review of something else (the review path reads only the working tree),
/// or a request to act on what the review finds: those are tasks for an agent.
const NOT: &[&str] = &[
    "pr",
    "pull request",
    "branch",
    "fix",
    "apply",
    "address",
    "resolve",
    "edit",
    "update",
    "rewrite",
    "refactor",
    "implement",
];

/// Whether `task` asks for a review of the working changes and nothing more.
pub(crate) fn asks_review(task: &str) -> bool {
    says_any(task, REVIEW) && says_any(task, CHANGES) && !says_any(task, NOT)
}

#[cfg(test)]
#[path = "reviewask_tests.rs"]
mod tests;
