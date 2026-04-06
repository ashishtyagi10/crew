use crate::types::FileEntry;
use std::collections::HashSet;
use std::path::PathBuf;

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
        };
        // Root is always expanded
        state.expanded.insert(root);
        state.rebuild();
        state
    }

    /// Rebuild the flattened visible_nodes from the tree structure
    pub fn rebuild(&mut self) {
        self.visible_nodes.clear();
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

        // Sort: directories first, then alphabetically
        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
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

    /// Change root directory
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root.clone();
        self.expanded.clear();
        self.expanded.insert(root);
        self.cursor = 0;
        self.scroll_offset = 0;
        self.selected.clear();
        self.filter.clear();
        self.rebuild();
    }
}
