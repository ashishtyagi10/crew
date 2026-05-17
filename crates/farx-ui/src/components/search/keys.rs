use super::action::{SearchAction, SearchField};
use super::state::SearchState;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

impl SearchState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> SearchAction {
        // If we have results and are browsing them
        if !self.results.is_empty() && self.field == SearchField::Pattern {
            if let Some(action) = self.handle_results_key(key) {
                return action;
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.active = false;
                SearchAction::Close
            }
            KeyCode::Tab => {
                self.field = match self.field {
                    SearchField::Pattern => SearchField::Content,
                    SearchField::Content => SearchField::Pattern,
                };
                SearchAction::None
            }
            KeyCode::Enter => {
                // Start search
                self.execute_search();
                SearchAction::None
            }
            KeyCode::Char(ch) => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern.insert(self.pattern_cursor, ch);
                        self.pattern_cursor += 1;
                    }
                    SearchField::Content => {
                        self.content_query.insert(self.content_cursor, ch);
                        self.content_cursor += 1;
                    }
                }
                SearchAction::None
            }
            KeyCode::Backspace => {
                match self.field {
                    SearchField::Pattern => {
                        if self.pattern_cursor > 0 {
                            self.pattern_cursor -= 1;
                            self.pattern.remove(self.pattern_cursor);
                        }
                    }
                    SearchField::Content => {
                        if self.content_cursor > 0 {
                            self.content_cursor -= 1;
                            self.content_query.remove(self.content_cursor);
                        }
                    }
                }
                SearchAction::None
            }
            KeyCode::Left => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern_cursor = self.pattern_cursor.saturating_sub(1);
                    }
                    SearchField::Content => {
                        self.content_cursor = self.content_cursor.saturating_sub(1);
                    }
                }
                SearchAction::None
            }
            KeyCode::Right => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern_cursor = (self.pattern_cursor + 1).min(self.pattern.len());
                    }
                    SearchField::Content => {
                        self.content_cursor =
                            (self.content_cursor + 1).min(self.content_query.len());
                    }
                }
                SearchAction::None
            }
            _ => SearchAction::None,
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                Some(SearchAction::Close)
            }
            KeyCode::Enter => {
                if !self.results.is_empty() {
                    let result = &self.results[self.result_cursor];
                    let path = if result.is_dir {
                        result.path.clone()
                    } else {
                        result.path.parent().unwrap_or(Path::new("/")).to_path_buf()
                    };
                    self.active = false;
                    return Some(SearchAction::GoTo(path));
                }
                None
            }
            KeyCode::Up => {
                if self.result_cursor > 0 {
                    self.result_cursor -= 1;
                    if self.result_cursor < self.result_scroll {
                        self.result_scroll = self.result_cursor;
                    }
                }
                Some(SearchAction::None)
            }
            KeyCode::Down => {
                if self.result_cursor + 1 < self.results.len() {
                    self.result_cursor += 1;
                }
                Some(SearchAction::None)
            }
            KeyCode::Tab => {
                // Clear results and go back to editing
                self.results.clear();
                self.result_cursor = 0;
                Some(SearchAction::None)
            }
            _ => None,
        }
    }
}
