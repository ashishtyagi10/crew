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

/// Whether the dispatcher answers `name` — the palette's rows PLUS the
/// spellings it no longer advertises.
///
/// The two are not the same list any more. `/look` folded fifteen appearance
/// commands into one row (`crate::lookcmd`), and every one of them still runs:
/// they are in every doc, every script and everyone's fingers. What the
/// palette offers is what you must KNOW; what the dispatcher answers is what
/// works. Surfaces that ask "is this real?" — the input bar's ink, the typo
/// note, the doc guards — must ask this, or typing `/theme dark` is marked as
/// a mistake while it runs perfectly.
pub(crate) fn answered(name: &str) -> bool {
    commands().any(|c| c.name == name) || unadvertised(name)
}

/// The names that run without a palette row of their own.
pub(crate) fn unadvertised(name: &str) -> bool {
    name.strip_prefix('/').is_some_and(crate::verbs::is_folded)
}

/// Whether any answered name STARTS with `part` — "you are on your way to
/// something", the middle state the input bar's ink draws. Unadvertised
/// names count here too: `/them` is on its way to `/theme` whether or not
/// the palette lists it.
pub(crate) fn answers_prefix(part: &str) -> bool {
    commands().any(|c| c.name.starts_with(part))
        || crate::lookcmd::SUBJECTS
            .iter()
            .map(|(s, _)| *s)
            .chain(crate::verbs::FOLDED.iter().copied())
            .any(|s| format!("/{s}").starts_with(part))
}

#[cfg(test)]
#[path = "parity_tests.rs"]
mod parity_tests;
