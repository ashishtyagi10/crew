//! Prefix history search for the input bar. Up/Down recall previous lines, but
//! when text is already typed they recall only history entries starting with it
//! (like zsh/fish history-search). An empty prefix matches everything, so a
//! blank input bar behaves like plain most-recent-first recall.
use crate::inputbar::InputBar;

/// Greatest index `< before` whose history entry starts with `prefix`.
pub(crate) fn prev_match(history: &[String], prefix: &str, before: usize) -> Option<usize> {
    (0..before).rev().find(|&i| history[i].starts_with(prefix))
}

/// Smallest index `> after` whose history entry starts with `prefix`.
pub(crate) fn next_match(history: &[String], prefix: &str, after: usize) -> Option<usize> {
    (after + 1..history.len()).find(|&i| history[i].starts_with(prefix))
}

impl InputBar {
    /// Recall an older history entry (Up) matching the typed prefix. The prefix
    /// is captured the first time navigation starts (when `hist_pos` is `None`).
    pub(crate) fn history_prev(&mut self) {
        if self.hist_pos.is_none() {
            self.hist_prefix = self.text.clone();
        }
        let start = self.hist_pos.unwrap_or(self.history.len());
        let prefix = self.hist_prefix.clone();
        if let Some(i) = prev_match(&self.history, &prefix, start) {
            self.hist_pos = Some(i);
            self.text = self.history[i].clone();
        }
    }

    /// Recall a newer matching entry (Down); past the newest, restore the prefix
    /// the user had typed and exit history navigation.
    pub(crate) fn history_next(&mut self) {
        let Some(cur) = self.hist_pos else {
            return;
        };
        let prefix = self.hist_prefix.clone();
        match next_match(&self.history, &prefix, cur) {
            Some(i) => {
                self.hist_pos = Some(i);
                self.text = self.history[i].clone();
            }
            None => {
                self.hist_pos = None;
                self.text = prefix;
            }
        }
    }
}

#[cfg(test)]
#[path = "histsearch_tests.rs"]
mod tests;
