use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PanelSide {
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SortField {
    #[default]
    Name,
    Extension,
    Size,
    Modified,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum PanelViewMode {
    Brief,
    Medium,
    #[default]
    Full,
    Wide,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Local>>,
    pub extension: Option<String>,
    pub readonly: bool,
}

pub struct PanelState {
    pub side: PanelSide,
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub selected: HashSet<usize>,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub view_mode: PanelViewMode,
    pub quick_search: Option<String>,
    pub column_widths: Vec<u16>,
}

impl PanelState {
    pub fn new(side: PanelSide, path: PathBuf) -> Self {
        Self {
            side,
            current_dir: path,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            sort_field: SortField::default(),
            sort_order: SortOrder::default(),
            view_mode: PanelViewMode::default(),
            quick_search: None,
            column_widths: Vec::new(),
        }
    }

    /// Returns the entry at the current cursor position, if any.
    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// Toggle selection at the current cursor position and move cursor down.
    pub fn toggle_select(&mut self) {
        if self.cursor < self.entries.len() {
            if self.selected.contains(&self.cursor) {
                self.selected.remove(&self.cursor);
            } else {
                self.selected.insert(self.cursor);
            }
            // Move cursor down after toggling
            if self.cursor + 1 < self.entries.len() {
                self.cursor += 1;
            }
        }
    }

    /// Toggle selection on current entry, then move cursor by delta.
    /// Used for Shift+Arrow selection.
    pub fn select_move(&mut self, delta: i32) {
        if self.cursor < self.entries.len() {
            // Skip ".." from selection
            if self.entries[self.cursor].name != ".." {
                if self.selected.contains(&self.cursor) {
                    self.selected.remove(&self.cursor);
                } else {
                    self.selected.insert(self.cursor);
                }
            }
        }
        self.move_cursor(delta);
    }

    /// Select range from current cursor to target position.
    /// Used for Shift+Home/End/PgUp/PgDn.
    pub fn select_range_to(&mut self, target: usize) {
        let target = target.min(self.entries.len().saturating_sub(1));
        let (from, to) = if target >= self.cursor {
            (self.cursor, target)
        } else {
            (target, self.cursor)
        };
        for i in from..=to {
            if i < self.entries.len() && self.entries[i].name != ".." {
                self.selected.insert(i);
            }
        }
        self.cursor = target;
    }

    /// Return references to all selected entries.
    pub fn selected_entries(&self) -> Vec<&FileEntry> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .into_iter()
            .filter_map(|i| self.entries.get(i))
            .collect()
    }

    /// Sort entries by the current sort_field and sort_order.
    /// Directories are always sorted before files.
    pub fn sort_entries(&mut self) {
        let sort_field = self.sort_field;
        let sort_order = self.sort_order;

        self.entries.sort_by(|a, b| {
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
    }

    /// Move cursor by a signed delta, clamping to valid range.
    pub fn move_cursor(&mut self, delta: i32) {
        if self.entries.is_empty() {
            self.cursor = 0;
            return;
        }
        let new_pos = self.cursor as i32 + delta;
        let max = self.entries.len().saturating_sub(1) as i32;
        self.cursor = new_pos.clamp(0, max) as usize;
    }

    /// Move cursor to an exact position, clamping to valid range.
    pub fn move_cursor_to(&mut self, pos: usize) {
        if self.entries.is_empty() {
            self.cursor = 0;
            return;
        }
        self.cursor = pos.min(self.entries.len().saturating_sub(1));
    }

    /// Adjust scroll_offset so the cursor remains visible within the given height.
    pub fn scroll_to_cursor(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    /// Append a character to the quick search filter and jump to matching entry.
    pub fn enter_quick_search(&mut self, ch: char) {
        let search = self.quick_search.get_or_insert_with(String::new);
        search.push(ch);
        let needle = search.to_lowercase();

        // Find first entry whose name starts with the search string
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.name.to_lowercase().starts_with(&needle))
        {
            self.cursor = pos;
        }
    }

    /// Clear the quick search filter.
    pub fn clear_quick_search(&mut self) {
        self.quick_search = None;
    }
}
