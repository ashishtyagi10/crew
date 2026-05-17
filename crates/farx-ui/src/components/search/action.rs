use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    None,
    Close,
    /// Navigate to the selected result
    GoTo(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SearchField {
    Pattern,
    Content,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// Matching lines from content search: (line_number, line_text).
    pub matching_lines: Vec<(usize, String)>,
}
