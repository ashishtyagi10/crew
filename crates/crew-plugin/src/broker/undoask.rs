//! The words that mean "put it back", matched exactly.
//!
//! Exact, not fuzzy, and not a model call: this is the gate in front of the
//! only path that writes over your files. "undo the nav change" is a TASK —
//! it asks an agent to make a change, not crew to revert one — and it must
//! reach the agents untouched. So the set is small, closed, and boring, like
//! the other human gates (`intent::gate`).

/// Whether `task` is asking for the last change to be taken back, and how far
/// — `Some(0)` is the newest checkpoint, `Some(2)` the third one down.
///
/// An exact match against a small set, like the other human gates: "undo the
/// nav change" is a TASK, not this, and must reach the agents untouched.
pub(crate) fn asks_undo(task: &str) -> Option<usize> {
    let t = task
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .to_ascii_lowercase();
    let (head, nth) = match t.rsplit_once(|c: char| c.is_whitespace()) {
        Some((h, n)) if n.starts_with('#') => (h.trim().to_string(), n[1..].parse().ok()),
        _ => (t.clone(), None),
    };
    let phrase = if nth.is_some() { head } else { t };
    let known = [
        "undo",
        "undo that",
        "undo it",
        "undo the last task",
        "undo the last change",
        "revert that",
        "revert it",
        "revert the last task",
        "put it back",
        "put that back",
        "roll it back",
        "roll that back",
        "take that back",
    ]
    .contains(&phrase.as_str());
    // `#1` is the newest, the way the list numbers it; internally it is 0.
    known.then(|| nth.map_or(0, |n: usize| n.saturating_sub(1)))
}

#[cfg(test)]
#[path = "undoask_tests.rs"]
mod tests;
