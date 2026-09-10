//! The `/disk` pane's key hint, chosen by width rather than cut.
//!
//! One 47-column string was put on the last row of a pane that draws from
//! 20 columns, and the put broke at the edge: a 30-column tile read
//! `←→ pick · enter opens · b`, which looks like a rendering fault. And
//! Esc closed the pane all along without the hint ever saying so.

/// The forms, longest first. Every one names `esc`: the way out is the
/// one key a hint must never drop.
pub(crate) const HINTS: &[&str] = &[
    "\u{2190}\u{2192} pick \u{00b7} enter opens \u{00b7} backspace up \u{00b7} r rescans \u{00b7} esc closes",
    "\u{2190}\u{2192} pick \u{00b7} enter opens \u{00b7} backspace up \u{00b7} esc",
    "\u{2190}\u{2192} \u{00b7} enter \u{00b7} backspace \u{00b7} esc",
    "\u{2190}\u{2192} enter esc",
];

/// The widest form that fits a `cols`-wide pane from column 1 with one
/// column of air at the right, or the shortest one.
pub(crate) fn hint(cols: u16) -> &'static str {
    let room = usize::from(cols.saturating_sub(2));
    HINTS
        .iter()
        .find(|s| crate::chatwidth::str_w(s) <= room)
        .or(HINTS.last())
        .copied()
        .unwrap_or("")
}

#[cfg(test)]
#[path = "diskhint_tests.rs"]
mod tests;
