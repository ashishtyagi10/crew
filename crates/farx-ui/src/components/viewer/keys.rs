use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Color;

use crate::components::markdown::render_markdown_with_bg;

use super::hex::hex_dump;
use super::state::ViewerState;
use super::ViewerAction;

impl ViewerState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ViewerAction {
        // Search input mode
        if let Some(ref mut input) = self.search_input {
            match key.code {
                KeyCode::Enter => {
                    let query = input.clone();
                    self.search_input = None;
                    self.find_in_viewer(&query);
                }
                KeyCode::Esc => {
                    self.search_input = None;
                }
                KeyCode::Char(ch) => {
                    input.push(ch);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            }
            return ViewerAction::None;
        }

        // Go-to-line input mode
        if let Some(ref mut input) = self.goto_input {
            match key.code {
                KeyCode::Enter => {
                    if let Ok(line_num) = input.parse::<usize>() {
                        self.scroll_offset = line_num
                            .saturating_sub(1)
                            .min(self.total_lines.saturating_sub(1));
                    }
                    self.goto_input = None;
                }
                KeyCode::Esc => {
                    self.goto_input = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    input.push(ch);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            }
            return ViewerAction::None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::F(10) | KeyCode::Char('q') => {
                self.active = false;
                ViewerAction::Close
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up(1);
                ViewerAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down(1);
                ViewerAction::None
            }
            KeyCode::PageUp => {
                self.scroll_up(30);
                ViewerAction::None
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll_down(30);
                ViewerAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                ViewerAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
                ViewerAction::None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.wrap = !self.wrap;
                ViewerAction::None
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.goto_input = Some(String::new());
                ViewerAction::None
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_markdown();
                ViewerAction::None
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_hex();
                ViewerAction::None
            }
            KeyCode::Char('/') | KeyCode::F(7) => {
                self.search_input = Some(String::new());
                ViewerAction::None
            }
            KeyCode::Char('n') => {
                self.find_next_in_viewer();
                ViewerAction::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.follow = !self.follow;
                if self.follow {
                    // Jump to end when enabling follow
                    self.scroll_offset =
                        self.total_lines.saturating_sub(self.visible_height.max(1));
                }
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }

    fn toggle_markdown(&mut self) {
        // Toggle markdown preview for .md files
        let ext = self.file_path.extension().and_then(|e| e.to_str());
        if matches!(ext, Some("md" | "markdown" | "mdx")) {
            self.markdown_mode = !self.markdown_mode;
            if self.markdown_mode && self.markdown_lines.is_empty() {
                // Re-render markdown
                let contents = self.lines.join("\n");
                self.markdown_lines = render_markdown_with_bg(&contents, Color::Rgb(22, 22, 26));
                self.total_lines = self.markdown_lines.len();
            } else if !self.markdown_mode {
                self.total_lines = self.lines.len();
            } else {
                self.total_lines = self.markdown_lines.len();
            }
            self.scroll_offset = 0;
        }
    }

    fn toggle_hex(&mut self) {
        if self.hex_mode {
            // Switch to text: re-read file as text
            if let Ok(text) = std::fs::read_to_string(&self.file_path) {
                self.lines = text.lines().map(String::from).collect();
                self.total_lines = self.lines.len();
                self.hex_mode = false;
                self.scroll_offset = 0;
            }
        } else {
            // Switch to hex: re-read file as bytes
            if let Ok(bytes) = std::fs::read(&self.file_path) {
                self.lines = hex_dump(&bytes);
                self.total_lines = self.lines.len();
                self.hex_mode = true;
                self.scroll_offset = 0;
            }
        }
    }
}
