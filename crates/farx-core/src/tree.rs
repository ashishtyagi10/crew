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

    /// Rebuild the flattened visible_nodes from the tree structure
    pub fn rebuild(&mut self) {
        self.visible_nodes.clear();

        // Add ".." entry at top if not at filesystem root
        if let Some(parent) = self.root.parent() {
            self.visible_nodes.push(TreeNode {
                entry: FileEntry {
                    name: "..".to_string(),
                    path: parent.to_path_buf(),
                    is_dir: true,
                    is_symlink: false,
                    is_hidden: false,
                    size: 0,
                    modified: None,
                    extension: None,
                    readonly: false,
                },
                depth: 0,
                expanded: false,
                has_children: true,
            });
        }

        self.build_tree(&self.root.clone(), 0);
        // Clamp cursor
        if self.cursor >= self.visible_nodes.len() {
            self.cursor = self.visible_nodes.len().saturating_sub(1);
        }
    }

    fn build_tree(&mut self, dir: &PathBuf, depth: usize) {
        let entries = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        let mut items: Vec<FileEntry> = Vec::new();
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files unless show_hidden is set
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let is_symlink = entry
                .path()
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false);
            let modified = metadata
                .modified()
                .ok()
                .map(chrono::DateTime::<chrono::Local>::from);
            let extension = if metadata.is_file() {
                entry
                    .path()
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
            } else {
                None
            };

            items.push(FileEntry {
                name,
                path: entry.path(),
                is_dir: metadata.is_dir(),
                is_symlink,
                is_hidden: false,
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                modified,
                extension,
                readonly: metadata.permissions().readonly(),
            });
        }

        // Sort: directories first, then by configured sort field/order
        let sort_field = self.sort_field;
        let sort_order = self.sort_order;
        items.sort_by(|a, b| {
            // Directories always come first
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            let ordering = match sort_field {
                SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortField::Extension => {
                    let ext_a = a.extension.as_deref().unwrap_or("");
                    let ext_b = b.extension.as_deref().unwrap_or("");
                    ext_a
                        .to_lowercase()
                        .cmp(&ext_b.to_lowercase())
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
                SortField::Size => a.size.cmp(&b.size),
                SortField::Modified => a.modified.cmp(&b.modified),
            };
            match sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });

        // Apply filter at depth 0 (root listing) — files only, dirs always pass
        let filter_lower = self.filter.to_lowercase();
        let items: Vec<FileEntry> = if !filter_lower.is_empty() && depth == 0 {
            items
                .into_iter()
                .filter(|item| item.is_dir || item.name.to_lowercase().contains(&filter_lower))
                .collect()
        } else {
            items
        };

        for item in items {
            let is_dir = item.is_dir;
            let path = item.path.clone();
            let is_expanded = self.expanded.contains(&path);

            let has_children = if is_dir {
                // Quick check if directory has any children
                std::fs::read_dir(&path)
                    .map(|mut rd| rd.next().is_some())
                    .unwrap_or(false)
            } else {
                false
            };

            self.visible_nodes.push(TreeNode {
                entry: item,
                depth,
                expanded: is_expanded,
                has_children,
            });

            // If expanded, recurse into children
            if is_dir && is_expanded {
                self.build_tree(&path, depth + 1);
            }
        }
    }

    /// Get current node
    pub fn current_node(&self) -> Option<&TreeNode> {
        self.visible_nodes.get(self.cursor)
    }

    /// Toggle expand/collapse of the directory at cursor
    pub fn toggle_expand(&mut self) {
        if let Some(node) = self.visible_nodes.get(self.cursor) {
            if node.entry.is_dir {
                let path = node.entry.path.clone();
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                self.rebuild();
            }
        }
    }

    /// Right arrow: expand collapsed dir, or move into first child of expanded dir.
    pub fn expand(&mut self) {
        let Some(node) = self.visible_nodes.get(self.cursor) else {
            return;
        };
        if !node.entry.is_dir {
            return;
        }

        let was_expanded = node.expanded;
        let depth = node.depth;

        if !was_expanded {
            let path = node.entry.path.clone();
            self.expanded.insert(path);
            self.rebuild();
        }

        // Move cursor to first child
        if self.cursor + 1 < self.visible_nodes.len()
            && self.visible_nodes[self.cursor + 1].depth > depth
        {
            self.cursor += 1;
        }
    }

    /// Left arrow: collapse expanded dir, or jump to parent node.
    pub fn collapse(&mut self) {
        if let Some(node) = self.visible_nodes.get(self.cursor) {
            if node.entry.is_dir && node.expanded {
                // Expanded → collapse
                let path = node.entry.path.clone();
                self.expanded.remove(&path);
                self.rebuild();
            } else {
                // Collapsed dir or file → jump to parent directory node
                let current_depth = node.depth;
                if current_depth > 0 {
                    for i in (0..self.cursor).rev() {
                        if self.visible_nodes[i].depth < current_depth
                            && self.visible_nodes[i].entry.is_dir
                        {
                            self.cursor = i;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let new_pos = (self.cursor as i32 + delta)
            .max(0)
            .min(self.visible_nodes.len() as i32 - 1) as usize;
        self.cursor = new_pos;
    }

    pub fn move_cursor_to(&mut self, pos: usize) {
        self.cursor = pos.min(self.visible_nodes.len().saturating_sub(1));
    }

    pub fn scroll_to_cursor(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        }
        if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn toggle_select(&mut self) {
        if self.cursor < self.visible_nodes.len() {
            // Skip ".." from selection
            if self.visible_nodes[self.cursor].entry.name == ".." {
                if self.cursor + 1 < self.visible_nodes.len() {
                    self.cursor += 1;
                }
                return;
            }
            if self.selected.contains(&self.cursor) {
                self.selected.remove(&self.cursor);
            } else {
                self.selected.insert(self.cursor);
            }
            if self.cursor + 1 < self.visible_nodes.len() {
                self.cursor += 1;
            }
        }
    }

    /// Change root directory (no history tracking — used for init/reset).
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root.clone();
        self.expanded.clear();
        self.expanded.insert(root);
        self.cursor = 0;
        self.scroll_offset = 0;
        self.selected.clear();
        self.filter.clear();
        self.rebuild();
        self.refresh_git_status();
    }

    /// Navigate to a new directory, pushing current location to history.
    pub fn navigate_to(&mut self, target: PathBuf) {
        if target == self.root {
            return;
        }
        // Push current to back-stack
        self.history_back.push(self.root.clone());
        // Clear forward stack on new navigation
        self.history_forward.clear();
        self.set_root(target);
    }

    /// Go back to the previous directory in history.
    /// Returns true if navigation occurred.
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.root.clone());
            self.set_root(prev);
            true
        } else {
            false
        }
    }

    /// Go forward in history.
    /// Returns true if navigation occurred.
    pub fn go_forward(&mut self) -> bool {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.root.clone());
            self.set_root(next);
            true
        } else {
            false
        }
    }

    /// Refresh git status for the current root directory.
    /// Runs `git status --porcelain` and parses per-file status.
    pub fn refresh_git_status(&mut self) {
        self.git_status.clear();
        self.in_git_repo = false;

        // Check if we're in a git repo by finding the git root
        let git_root = match std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&self.root)
            .output()
        {
            Ok(out) if out.status.success() => {
                let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
                PathBuf::from(root)
            }
            _ => return,
        };

        self.in_git_repo = true;

        let output = match std::process::Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(&git_root)
            .output()
        {
            Ok(out) if out.status.success() => out.stdout,
            _ => return,
        };

        let text = String::from_utf8_lossy(&output);
        for line in text.lines() {
            if line.len() < 4 {
                continue;
            }
            let xy = &line[..2];
            let path_str = &line[3..];
            // Handle renames: "R  old -> new"
            let file_path = if let Some(arrow) = path_str.find(" -> ") {
                &path_str[arrow + 4..]
            } else {
                path_str
            };
            let abs_path = git_root.join(file_path);
            let status = match xy {
                "??" => GitFileStatus::Untracked,
                "!!" => GitFileStatus::Ignored,
                "UU" | "AA" | "DD" => GitFileStatus::Conflict,
                _ => {
                    let index = xy.as_bytes()[0];
                    let worktree = xy.as_bytes()[1];
                    if index == b'R' || worktree == b'R' {
                        GitFileStatus::Renamed
                    } else if index == b'D' || worktree == b'D' {
                        GitFileStatus::Deleted
                    } else if index != b' ' && index != b'?' {
                        GitFileStatus::Staged
                    } else if worktree == b'M' || worktree == b'A' {
                        GitFileStatus::Modified
                    } else {
                        continue;
                    }
                }
            };
            self.git_status.insert(abs_path, status);
        }
    }

    /// Get the git status for a given absolute path.
    pub fn git_status_for(&self, path: &PathBuf) -> Option<GitFileStatus> {
        self.git_status.get(path).copied()
    }
}
