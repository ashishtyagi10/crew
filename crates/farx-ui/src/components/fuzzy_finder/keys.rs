use crossterm::event::{KeyCode, KeyEvent};

use super::state::{FuzzyAction, FuzzyFinderState};

impl FuzzyFinderState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> FuzzyAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                FuzzyAction::Close
            }
            KeyCode::Enter => {
                if let Some(result) = self.results.get(self.result_cursor) {
                    let path = result.path.clone();
                    self.active = false;
                    FuzzyAction::GoTo(path)
                } else {
                    FuzzyAction::None
                }
            }
            KeyCode::Up => {
                if self.result_cursor > 0 {
                    self.result_cursor -= 1;
                    if self.result_cursor < self.result_scroll {
                        self.result_scroll = self.result_cursor;
                    }
                }
                FuzzyAction::None
            }
            KeyCode::Down => {
                if self.result_cursor + 1 < self.results.len() {
                    self.result_cursor += 1;
                }
                FuzzyAction::None
            }
            KeyCode::Char(ch) => {
                self.query.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
                self.update_results();
                FuzzyAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                    self.update_results();
                }
                FuzzyAction::None
            }
            _ => FuzzyAction::None,
        }
    }
}
