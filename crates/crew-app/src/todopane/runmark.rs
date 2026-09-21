//! The trailing `&` that runs a todo the moment it is added.
//!
//! `r` on a row was the one deliberate act that started a run, and the
//! user asked for the other reading as well — an item tagged for a project
//! should go straight to an agent. The two readings coexist by a spelling
//! every shell already taught: a trailing `&` means "start this and give me
//! the prompt back". `fix the flaky test @crew &` is added AND handed off
//! on one Enter; `fix the flaky test @crew` is a todo, as before. The mark
//! is the draft's last non-blank char, so `a && b`, `R&D` and a URL keep
//! their ampersands.
pub(crate) const MARK: char = '&';

/// The draft without its run mark, and whether one was there.
pub(crate) fn strip(text: &str) -> (&str, bool) {
    match text.trim_end().strip_suffix(MARK) {
        Some(rest) => (rest.trim_end(), true),
        None => (text, false),
    }
}

/// The char index of the mark in `chars` — what the composer tints so the
/// draft says it will run before Enter does.
pub(crate) fn at(chars: &[char]) -> Option<usize> {
    let last = chars.iter().rposition(|c| !c.is_whitespace())?;
    (chars[last] == MARK).then_some(last)
}

#[cfg(test)]
#[path = "runmark_tests.rs"]
mod tests;
