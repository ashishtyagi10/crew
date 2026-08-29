//! The command palette's table: every slash command with its palette
//! description, in priority order (prefix ties break by list position).
//! Logic lives in `suggest`.
//!
//! The table is data, and it is the one part of crew that grows by a row
//! every time a command is added — so it lives in [`work`] and [`look`], and
//! this file holds the shape of a row and the order the two are read in.
//!
//! Split across files, NOT into independent tables: the palette's tie-break
//! IS list position, so [`commands`] concatenates the groups in a fixed order
//! and every reader walks that one sequence. Where a row lives is a question
//! about file length; where it ranks is a question about the palette, and the
//! two must not become the same question.
mod look;
mod work;

/// A slash command shown in the command palette.
pub(crate) struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
}

/// The groups, in palette priority order.
const GROUPS: &[&[Cmd]] = &[work::WORK, look::LOOK];

/// Every slash command the palette offers, highest priority first.
pub(crate) fn commands() -> impl Iterator<Item = &'static Cmd> {
    GROUPS.iter().copied().flatten()
}

#[cfg(test)]
#[path = "parity_tests.rs"]
mod parity_tests;
