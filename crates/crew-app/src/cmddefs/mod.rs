//! The command palette's table: every slash command with its palette
//! description, in priority order (prefix ties break by list position).
//! Logic lives in `suggest`.
//!
//! The table is data, and it is the one part of crew that grows by a row
//! every time a command is added — so it lives in [`list`] on its own, and
//! this file holds the shape of a row. Splitting the file, not the array,
//! keeps `COMMANDS` a single ordered slice: the palette's tie-break IS list
//! position, and a table assembled from parts would make that ordering a
//! property of how the file happened to be cut up.
mod list;

pub(crate) use list::COMMANDS;

/// A slash command shown in the command palette.
pub(crate) struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
}
