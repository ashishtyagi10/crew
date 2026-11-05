//! The ink an ALERT toast is drawn in, by what it is about.
//!
//! Every alert card used to take the bell's red: `waiting` (an agent has a
//! question for you) wore the same stroke and legend as `failed` and
//! `exited`, so a card asking for an answer read as one reporting a break.
//! Red is kept for what went wrong; a card that wants you back without
//! anything having failed takes the attention ink (`ansi[11]`), the one the
//! footer's `allow?` already asks in. Split from [`crate::toastcard`], whose
//! line budget is spent, and from [`crate::toast`], which is in cap debt.

/// Legends that ask for the user rather than report a failure.
const ASKS: &[&str] = &["waiting"];

/// The stroke and legend ink of an alert card with `legend`. Taken from the
/// theme's ANSI table, so a phosphor tube asks in its own phosphor and paper
/// asks in an amber dark enough to read.
pub(crate) fn ink(legend: &str) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    match ASKS.contains(&legend) {
        true => t.ansi[11],
        false => t.bell,
    }
}

#[cfg(test)]
#[path = "toastalert_tests.rs"]
mod tests;
