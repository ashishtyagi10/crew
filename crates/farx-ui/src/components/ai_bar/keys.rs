use super::clipboard::copy_to_clipboard;
use super::state::{AiBarAction, AiBarState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl AiBarState {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> AiBarAction {
        if self.thinking {
            // While thinking, only allow Esc to cancel
            if key.code == KeyCode::Esc {
                self.active = false;
                return AiBarAction::Close;
            }
            return AiBarAction::None;
        }

        // Ctrl+C: copy response to clipboard
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.response.is_empty() {
                let text = self.response.join("\n");
                copy_to_clipboard(&text);
                self.copied = true;
            }
            return AiBarAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.active = false;
                AiBarAction::Close
            }
            KeyCode::Enter => {
                if self.input.is_empty() {
                    AiBarAction::None
                } else {
                    let query = self.input.clone();
                    self.thinking = true;
                    self.copied = false;
                    AiBarAction::Submit(query)
                }
            }
            KeyCode::Char(ch) => {
                self.input.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
                AiBarAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
                AiBarAction::None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
                AiBarAction::None
            }
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
                AiBarAction::None
            }
            KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.input.len());
                AiBarAction::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                AiBarAction::None
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                AiBarAction::None
            }
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                AiBarAction::None
            }
            KeyCode::Down => {
                self.scroll_offset += 1;
                AiBarAction::None
            }
            _ => AiBarAction::None,
        }
    }
}
