//! Input editing: character insertion, backspace, clear, suggestion accept.

use super::state::CommandLineState;

impl CommandLineState {
    /// Insert a character at the current cursor position.
    pub fn input_char(&mut self, ch: char) {
        self.input.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        self.invalidate_suggestion();
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
            self.invalidate_suggestion();
        }
    }

    /// Clear all input text.
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.suggestion = None;
        self.suggestion_pending = false;
    }

    /// Accept the current suggestion (Tab key).
    pub fn accept_suggestion(&mut self) -> bool {
        if let Some(suggestion) = self.suggestion.take() {
            self.input.push_str(&suggestion);
            self.cursor_pos = self.input.len();
            self.suggestion_pending = false;
            true
        } else {
            false
        }
    }

    /// Mark suggestion as stale after typing.
    fn invalidate_suggestion(&mut self) {
        self.suggestion = None;
        self.suggestion_pending = false;
        self.last_typed_tick = 0; // will be set by tick()
    }

    /// Take the current input, clearing the state, and return it.
    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        self.history_index = None;
        input
    }
}
