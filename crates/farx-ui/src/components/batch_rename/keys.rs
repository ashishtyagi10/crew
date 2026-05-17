use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use super::state::{ActiveField, BatchRenameAction, BatchRenameState};

impl BatchRenameState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> BatchRenameAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                return BatchRenameAction::Close;
            }
            KeyCode::Tab => {
                self.field = match self.field {
                    ActiveField::Find => ActiveField::Replace,
                    ActiveField::Replace => ActiveField::Find,
                };
                return BatchRenameAction::None;
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || self.field == ActiveField::Replace
                {
                    // Apply renames
                    let renames: Vec<(PathBuf, String)> = self
                        .files
                        .iter()
                        .zip(self.previews.iter())
                        .filter(|((_, old), new)| old != *new)
                        .map(|((path, _), new)| (path.clone(), new.clone()))
                        .collect();
                    self.active = false;
                    return BatchRenameAction::Apply(renames);
                }
                // Enter in find field moves to replace
                self.field = ActiveField::Replace;
                return BatchRenameAction::None;
            }
            KeyCode::Char(ch) => {
                match self.field {
                    ActiveField::Find => {
                        self.find_pattern.insert(self.find_cursor, ch);
                        self.find_cursor += 1;
                    }
                    ActiveField::Replace => {
                        self.replace_pattern.insert(self.replace_cursor, ch);
                        self.replace_cursor += 1;
                    }
                }
                self.update_previews();
                return BatchRenameAction::None;
            }
            KeyCode::Backspace => {
                match self.field {
                    ActiveField::Find => {
                        if self.find_cursor > 0 {
                            self.find_cursor -= 1;
                            self.find_pattern.remove(self.find_cursor);
                        }
                    }
                    ActiveField::Replace => {
                        if self.replace_cursor > 0 {
                            self.replace_cursor -= 1;
                            self.replace_pattern.remove(self.replace_cursor);
                        }
                    }
                }
                self.update_previews();
                return BatchRenameAction::None;
            }
            KeyCode::Left => match self.field {
                ActiveField::Find => {
                    self.find_cursor = self.find_cursor.saturating_sub(1);
                }
                ActiveField::Replace => {
                    self.replace_cursor = self.replace_cursor.saturating_sub(1);
                }
            },
            KeyCode::Right => match self.field {
                ActiveField::Find => {
                    self.find_cursor = (self.find_cursor + 1).min(self.find_pattern.len());
                }
                ActiveField::Replace => {
                    self.replace_cursor = (self.replace_cursor + 1).min(self.replace_pattern.len());
                }
            },
            _ => {}
        }
        BatchRenameAction::None
    }
}
