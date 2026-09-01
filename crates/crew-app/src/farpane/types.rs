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

/// An in-pane single-line text prompt — currently only "make folder" (F7).
pub(crate) struct Prompt {
    pub kind: PromptKind,
    pub input: String,
}

#[derive(Clone, Copy)]
pub(crate) enum PromptKind {
    MkDir,
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
