use super::{EditorAction, EditorState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl EditorState {
    pub(super) fn handle_nav_or_edit(&mut self, key: KeyEvent) -> EditorAction {
        match (key.code, key.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_col();
                }
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_cursor_col();
                }
            }
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    let line = &self.lines[self.cursor_line];
                    while self.cursor_col > 0 && !line.is_char_boundary(self.cursor_col) {
                        self.cursor_col -= 1;
                    }
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.current_line_len();
                }
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                if self.cursor_col < self.current_line_len() {
                    self.cursor_col += 1;
                    let line = &self.lines[self.cursor_line];
                    while self.cursor_col < line.len() && !line.is_char_boundary(self.cursor_col) {
                        self.cursor_col += 1;
                    }
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            (KeyCode::Home, KeyModifiers::NONE) => self.cursor_col = 0,
            (KeyCode::End, KeyModifiers::NONE) => self.cursor_col = self.current_line_len(),
            (KeyCode::Home, KeyModifiers::CONTROL) => {
                self.cursor_line = 0;
                self.cursor_col = 0;
            }
            (KeyCode::End, KeyModifiers::CONTROL) => {
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.current_line_len();
            }
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.cursor_line = self.cursor_line.saturating_sub(30);
                self.clamp_cursor_col();
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.cursor_line = (self.cursor_line + 30).min(self.lines.len().saturating_sub(1));
                self.clamp_cursor_col();
            }
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => self.insert_char(ch),
            (KeyCode::Enter, KeyModifiers::NONE) => self.insert_newline(),
            (KeyCode::Backspace, _) => self.backspace(),
            (KeyCode::Delete, _) => self.delete_char(),
            (KeyCode::Tab, KeyModifiers::NONE) => {
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            _ => {}
        }
        EditorAction::None
    }
}
