use super::{EditorAction, EditorMode, EditorState};
use crate::components::markdown::render_markdown_with_bg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Color;

impl EditorState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> EditorAction {
        // Markdown preview mode — limited key handling
        if self.preview_mode {
            match key.code {
                KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview_mode = false;
                }
                KeyCode::Esc => {
                    self.preview_mode = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.preview_scroll = self.preview_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(30);
                }
                KeyCode::PageDown | KeyCode::Char(' ') => {
                    self.preview_scroll = self.preview_scroll.saturating_add(30);
                }
                KeyCode::Home => {
                    self.preview_scroll = 0;
                }
                KeyCode::End => {
                    self.preview_scroll = self.preview_lines.len().saturating_sub(1);
                }
                _ => {}
            }
            return EditorAction::None;
        }

        match self.mode {
            EditorMode::ConfirmExit => return self.handle_confirm_exit(key),
            EditorMode::Search => return self.handle_search(key),
            EditorMode::GotoLine => return self.handle_goto_line(key),
            EditorMode::Normal => {}
        }

        self.handle_normal_key(key)
    }

    fn handle_confirm_exit(&mut self, key: KeyEvent) -> EditorAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                self.active = false;
                return EditorAction::Close;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if self.save().is_ok() {
                    self.active = false;
                    return EditorAction::SaveAndClose;
                }
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
        EditorAction::None
    }

    fn handle_search(&mut self, key: KeyEvent) -> EditorAction {
        match key.code {
            KeyCode::Enter => {
                self.mode = EditorMode::Normal;
                self.find_next();
            }
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            KeyCode::Char(ch) => {
                self.search_query.insert(self.search_cursor, ch);
                self.search_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                }
            }
            KeyCode::Left => {
                self.search_cursor = self.search_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.search_cursor = (self.search_cursor + 1).min(self.search_query.len());
            }
            _ => {}
        }
        EditorAction::None
    }

    fn handle_goto_line(&mut self, key: KeyEvent) -> EditorAction {
        match key.code {
            KeyCode::Enter => {
                self.mode = EditorMode::Normal;
                if let Ok(line_num) = self.goto_line_input.parse::<usize>() {
                    let target = line_num
                        .saturating_sub(1)
                        .min(self.lines.len().saturating_sub(1));
                    self.cursor_line = target;
                    self.cursor_col = 0;
                }
                self.goto_line_input.clear();
            }
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.goto_line_input.clear();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                self.goto_line_input.push(ch);
            }
            KeyCode::Backspace => {
                self.goto_line_input.pop();
            }
            _ => {}
        }
        EditorAction::None
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> EditorAction {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::F(10), _) => {
                if self.modified {
                    self.mode = EditorMode::ConfirmExit;
                } else {
                    self.active = false;
                    return EditorAction::Close;
                }
            }
            (KeyCode::F(2), KeyModifiers::NONE) | (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                let _ = self.save();
            }
            (KeyCode::F(2), KeyModifiers::SHIFT) | (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                if self.save().is_ok() {
                    self.active = false;
                    return EditorAction::SaveAndClose;
                }
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.wrap = !self.wrap;
            }
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                if self.is_markdown_file() {
                    let contents = self.lines.join("\n");
                    self.preview_lines = render_markdown_with_bg(&contents, Color::Rgb(22, 22, 26));
                    self.preview_scroll = 0;
                    self.preview_mode = true;
                }
            }
            (KeyCode::F(7), KeyModifiers::NONE) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::Search;
                self.search_cursor = self.search_query.len();
            }
            (KeyCode::F(3), KeyModifiers::NONE) => {
                self.find_next();
            }
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::GotoLine;
                self.goto_line_input.clear();
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                self.undo();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.redo();
            }
            _ => return self.handle_nav_or_edit(key),
        }
        EditorAction::None
    }
}
