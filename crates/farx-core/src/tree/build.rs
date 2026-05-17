use super::types::{TreeNode, TreeState};
use crate::types::{FileEntry, SortField, SortOrder};
use std::path::PathBuf;

impl TreeState {
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
                    mode: None,
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

    pub(super) fn build_tree(&mut self, dir: &PathBuf, depth: usize) {
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
                mode: {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        Some(metadata.permissions().mode())
                    }
                    #[cfg(not(unix))]
                    {
                        None
                    }
                },
            });
        }

        sort_items(&mut items, self.sort_field, self.sort_order);

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
}

/// Sort: directories first, then by configured sort field/order
fn sort_items(items: &mut [FileEntry], sort_field: SortField, sort_order: SortOrder) {
    items.sort_by(|a, b| {
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
}
