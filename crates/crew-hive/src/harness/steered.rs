//! Where a steer lands in a swarm worker's requests (see [`crate::steers`]).
//!
//! Text rounds rebuild the prompt every round, so each follow-up carries the
//! whole log after the tool exchanges, where the relay puts it. Native rounds
//! resend their turns whole, so only what is new since the last round goes in,
//! as a user turn after the tool results. A worker that starts after a steer
//! finds it at the end of its first prompt.

use crate::provider::Turn;
use crate::steers::{section, Steers, WORKER_TAIL};

/// The section for everything this run was told so far; empty with no steers.
pub(super) fn heard(steers: Option<&Steers>) -> String {
    steers.map_or_else(String::new, |s| section(&s.all(), WORKER_TAIL))
}

/// `prompt` with what the run was told before this worker started at its
/// end, and how many steers that was.
pub(super) fn opening(prompt: String, steers: Option<&Steers>) -> (String, usize) {
    let Some(s) = steers else {
        return (prompt, 0);
    };
    let all = s.all();
    if all.is_empty() {
        return (prompt, 0);
    }
    let text = format!("{prompt}\n\n{}", section(&all, WORKER_TAIL).trim_end());
    (text, all.len())
}

/// A user turn with the steers after the first `taken`, when there are any.
pub(super) fn turn(steers: Option<&Steers>, taken: &mut usize) -> Option<Turn> {
    let all = steers?.all();
    let new = all.get(*taken..).filter(|n| !n.is_empty())?;
    let text = section(new, WORKER_TAIL).trim_end().to_string();
    *taken = all.len();
    Some(Turn::User(text))
}

#[cfg(test)]
#[path = "steered_tests.rs"]
mod tests;
