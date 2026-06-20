//! Two small private enums used by the App: pending input-dialog operations
//! and the undo stack entry type.

use std::path::PathBuf;

/// Pending operation for input dialogs (MkDir, Rename, CreateFile).
#[derive(Debug, Clone)]
pub(super) enum PendingOperation {
    MkDir { parent: PathBuf },
    Rename { original: PathBuf },
    CreateFile { parent: PathBuf },
    CopySameDir { source: PathBuf },
    SelectByMask,
    DeselectByMask,
    CreateSymlink { target: PathBuf },
    GotoDirectory,
}

/// A recorded file operation that can be undone.
#[derive(Debug, Clone)]
pub(super) enum UndoEntry {
    /// Files were deleted (moved to trash). Record paths for feedback.
    Delete { paths: Vec<PathBuf> },
    /// Files were moved from sources to dest dir.
    Move {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
    /// A file was renamed from old to new.
    Rename { old: PathBuf, new: PathBuf },
}
