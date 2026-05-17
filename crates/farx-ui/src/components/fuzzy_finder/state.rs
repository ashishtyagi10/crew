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
#[allow(dead_code)]
pub(crate) struct FuzzyResult {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
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
    #[allow(dead_code)]
    pub(crate) root: PathBuf,
    pub(super) all_files: Vec<(PathBuf, String, String)>, // (abs_path, name, rel_path)
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
            root: root.clone(),
            all_files: Vec::new(),
        };
        scan_files(&mut state.all_files, &root, &root, 0);
        state.results = state
            .all_files
            .iter()
            .take(100)
            .map(|(p, n, r)| FuzzyResult {
                path: p.clone(),
                name: n.clone(),
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
                .map(|(p, n, r)| FuzzyResult {
                    path: p.clone(),
                    name: n.clone(),
                    rel_path: r.clone(),
                    score: 0,
                })
                .collect();
        } else {
            let query_lower = self.query.to_lowercase();
            let mut scored: Vec<FuzzyResult> = self
                .all_files
                .iter()
                .filter_map(|(p, n, r)| {
                    let score = fuzzy_score(&r.to_lowercase(), &query_lower);
                    if score > 0 {
                        Some(FuzzyResult {
                            path: p.clone(),
                            name: n.clone(),
                            rel_path: r.clone(),
                            score,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by(|a, b| b.score.cmp(&a.score));
            scored.truncate(100);
            self.results = scored;
        }
        self.result_cursor = 0;
        self.result_scroll = 0;
    }
}
