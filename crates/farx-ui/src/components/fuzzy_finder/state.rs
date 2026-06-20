use std::path::PathBuf;

use super::scoring::fuzzy_score;
use super::walker::scan_files;

#[derive(Debug, Clone, PartialEq)]
pub enum FuzzyAction {
    None,
    Close,
    /// Navigate to this path's parent directory
    GoTo(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) struct FuzzyResult {
    pub(crate) path: PathBuf,
    pub(crate) rel_path: String,
    pub(crate) score: i32,
}

pub struct FuzzyFinderState {
    pub active: bool,
    pub query: String,
    pub cursor_pos: usize,
    #[allow(private_interfaces)]
    pub results: Vec<FuzzyResult>,
    pub result_cursor: usize,
    pub result_scroll: usize,
    pub(super) all_files: Vec<(PathBuf, String)>, // (abs_path, rel_path)
}

impl FuzzyFinderState {
    pub fn new(root: PathBuf) -> Self {
        let mut state = Self {
            active: true,
            query: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            result_cursor: 0,
            result_scroll: 0,
            all_files: Vec::new(),
        };
        scan_files(&mut state.all_files, &root, &root, 0);
        state.results = state
            .all_files
            .iter()
            .take(100)
            .map(|(p, r)| FuzzyResult {
                path: p.clone(),
                rel_path: r.clone(),
                score: 0,
            })
            .collect();
        state
    }

    pub(super) fn update_results(&mut self) {
        if self.query.is_empty() {
            self.results = self
                .all_files
                .iter()
                .take(100)
                .map(|(p, r)| FuzzyResult {
                    path: p.clone(),
                    rel_path: r.clone(),
                    score: 0,
                })
                .collect();
        } else {
            let query_lower = self.query.to_lowercase();
            let mut scored: Vec<FuzzyResult> = self
                .all_files
                .iter()
                .filter_map(|(p, r)| {
                    let score = fuzzy_score(&r.to_lowercase(), &query_lower);
                    if score > 0 {
                        Some(FuzzyResult {
                            path: p.clone(),
                            rel_path: r.clone(),
                            score,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by_key(|r| std::cmp::Reverse(r.score));
            scored.truncate(100);
            self.results = scored;
        }
        self.result_cursor = 0;
        self.result_scroll = 0;
    }
}
