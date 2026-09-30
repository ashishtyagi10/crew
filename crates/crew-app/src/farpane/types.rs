//! The `/far` pane's own vocabulary: which side a panel is, one entry in a
//! listing, and the prompt a key press can raise.
//!
//! Split out of [`super`] for the line cap — and for headroom. This is the
//! third sibling split that pushed mod.rs one line over by adding a `mod`
//! declaration to it; a file that is mostly a manifest has nowhere to give.
use super::location::Location;

/// Which panel currently has the cursor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

/// One filesystem entry shown in a panel.
pub(crate) struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// The synthetic ".." row that ascends to the parent directory.
    pub is_parent: bool,
    /// File size in bytes; 0 for directories and the parent row.
    pub size: u64,
}

/// An in-pane single-line text prompt: F7's "make folder" on the bottom row,
/// or F6's "rename or move" box over the panels.
pub(crate) struct Prompt {
    pub kind: PromptKind,
    pub input: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum PromptKind {
    MkDir,
    /// F6 on `name`: `input` starts as `offered`, the other panel's folder —
    /// Enter as it stands moves there, a new name renames in place (`moveto`).
    Move {
        name: String,
        is_dir: bool,
        offered: String,
    },
}

impl Prompt {
    /// F6's box floats over the panels; F7's prompt takes the key row.
    pub(crate) fn floats(&self) -> bool {
        matches!(self.kind, PromptKind::Move { .. })
    }
}

/// One side of the dual-pane manager: a location and its sorted listing.
pub(crate) struct Panel {
    pub loc: Location,
    pub entries: Vec<Entry>,
    pub sel: usize,
    /// True while a remote listing (`rclone lsjson`) is in flight for this
    /// side — cleared by `FarPane::absorb_list`. Always `false` for local
    /// panels, which reload synchronously.
    pub loading: bool,
}
