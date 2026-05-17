use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum BatchRenameAction {
    None,
    Close,
    /// Apply the renames: Vec<(old_path, new_name)>
    Apply(Vec<(PathBuf, String)>),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ActiveField {
    Find,
    Replace,
}

pub struct BatchRenameState {
    pub active: bool,
    pub(super) field: ActiveField,
    pub find_pattern: String,
    pub replace_pattern: String,
    pub(super) find_cursor: usize,
    pub(super) replace_cursor: usize,
    /// Original file paths and names
    pub files: Vec<(PathBuf, String)>,
    /// Preview of new names (computed from find/replace)
    pub previews: Vec<String>,
    pub scroll: usize,
}

impl BatchRenameState {
    pub fn new(files: Vec<(PathBuf, String)>) -> Self {
        let previews = files.iter().map(|(_, n)| n.clone()).collect();
        Self {
            active: true,
            field: ActiveField::Find,
            find_pattern: String::new(),
            replace_pattern: String::new(),
            find_cursor: 0,
            replace_cursor: 0,
            files,
            previews,
            scroll: 0,
        }
    }
}
