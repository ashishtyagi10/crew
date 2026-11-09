//! What a surface's stop check made of an answer ([`super::Tools::stop`]).
//!
//! A turn ends when the model answers without asking for a tool, and until
//! now nothing of the user's own could say "not yet": a project rule such as
//! "the tests pass" or "no TODO left" could only be hoped for. Claude Code,
//! Codex and Kiro all let a `Stop` hook hold a turn open with a reason; this
//! is that seam, kept apart from how the surface decides (crew-plugin runs
//! the user's `hooks.json`), so the harness only has to act on the verdict.

/// What the stop check said about one answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopCheck {
    /// Let the turn end. Also what a surface with no check says, silently.
    End,
    /// Keep working: the reason is fed back to the model as a note of its
    /// own, and the turn goes on (the harness caps how often).
    Hold(String),
    /// A check could not run or did not finish, and was not set to hold the
    /// turn when it cannot: the turn ends, and this says why to the person.
    Broke(String),
}
