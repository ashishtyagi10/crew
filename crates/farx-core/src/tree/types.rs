use crate::types::{FileEntry, SortField, SortOrder};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Git status for a single file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    Modified,
    Staged,
    Untracked,
    Conflict,
    Deleted,
    Renamed,
    Ignored,
}

/// A node in the file tree
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: FileEntry,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
}

/// Tree state that manages expanded directories and the flattened view
pub struct TreeState {
    /// Root directory
    pub root: PathBuf,
    /// Set of expanded directory paths
    pub expanded: HashSet<PathBuf>,
    /// Flattened list of visible nodes (rebuilt when tree changes)
    pub visible_nodes: Vec<TreeNode>,
    /// Cursor position in the visible list
    pub cursor: usize,
    /// Scroll offset
    pub scroll_offset: usize,
    /// Selected node indices
    pub selected: HashSet<usize>,
    /// Whether to show hidden files
    pub show_hidden: bool,
    /// Active filter pattern (empty = no filter)
    pub filter: String,
    /// Navigation history — directories visited (back stack)
    pub history_back: Vec<PathBuf>,
    /// Navigation history — directories for forward navigation
    pub history_forward: Vec<PathBuf>,
    /// Per-file git status (path relative to git root → status).
    pub git_status: HashMap<PathBuf, GitFileStatus>,
    /// Whether we are inside a git repository.
    pub in_git_repo: bool,
    /// Sort field for file ordering.
    pub sort_field: SortField,
    /// Sort order (ascending/descending).
    pub sort_order: SortOrder,
}

impl TreeState {
    pub fn new(root: PathBuf) -> Self {
        let mut state = Self {
            root: root.clone(),
            expanded: HashSet::new(),
            visible_nodes: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            show_hidden: false,
            filter: String::new(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            git_status: HashMap::new(),
            in_git_repo: false,
            sort_field: SortField::default(),
            sort_order: SortOrder::default(),
        };
        // Root is always expanded
        state.expanded.insert(root);
        state.rebuild();
        state.refresh_git_status();
        state
    }
}
