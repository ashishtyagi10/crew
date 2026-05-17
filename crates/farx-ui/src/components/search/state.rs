use super::action::{SearchField, SearchResult};
use super::walker::search_recursive;
use std::path::PathBuf;

pub struct SearchState {
    pub active: bool,
    pub(crate) field: SearchField,
    pub pattern: String,
    pub content_query: String,
    pub(crate) pattern_cursor: usize,
    pub(crate) content_cursor: usize,
    pub results: Vec<SearchResult>,
    pub result_cursor: usize,
    pub result_scroll: usize,
    pub searching: bool,
    pub search_dir: PathBuf,
}

impl SearchState {
    pub fn new(search_dir: PathBuf) -> Self {
        Self {
            active: true,
            field: SearchField::Pattern,
            pattern: "*".to_string(),
            content_query: String::new(),
            pattern_cursor: 1,
            content_cursor: 0,
            results: Vec::new(),
            result_cursor: 0,
            result_scroll: 0,
            searching: false,
            search_dir,
        }
    }

    /// Create a search dialog with content field focused (for /grep).
    pub fn new_content_focused(search_dir: PathBuf) -> Self {
        Self {
            active: true,
            field: SearchField::Content,
            pattern: "*".to_string(),
            content_query: String::new(),
            pattern_cursor: 1,
            content_cursor: 0,
            results: Vec::new(),
            result_cursor: 0,
            result_scroll: 0,
            searching: false,
            search_dir,
        }
    }

    pub(crate) fn execute_search(&mut self) {
        self.results.clear();
        self.result_cursor = 0;
        self.result_scroll = 0;
        self.searching = true;

        let pattern = self.pattern.clone();
        let content_query = self.content_query.clone();
        let search_dir = self.search_dir.clone();

        // Synchronous search (could be made async later)
        search_recursive(
            &search_dir,
            &pattern,
            &content_query,
            &mut self.results,
            5000,
        );

        self.searching = false;
    }
}
