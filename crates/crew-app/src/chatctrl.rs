//! The control keys a chat composer answers to, the way terminal agents do.
//!
//! Ctrl+C clears what is typed — or, on an empty composer while a turn runs,
//! interrupts it, as Esc does. Ctrl+D on an empty, idle composer closes the
//! pane. Ctrl+J, and a `\` just before Enter, insert a newline instead of
//! sending. `?` on an empty composer opens the key reference. Any other
//! Ctrl+letter does nothing: they all used to type their letter.
use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};

impl ChatPane {
    /// The keys that act before any popup or the composer sees them — a
    /// waiting approval's and Shift+Tab's first (`chatapprove`). `Some` when
    /// consumed, carrying what `on_input` returns.
    pub(crate) fn first_keys(
        &mut self,
        k: &ChatInput,
        cwd: &std::path::Path,
    ) -> Option<Option<ChatAction>> {
        if let Some(done) = self.approval_key(k).or_else(|| self.slash_key(k)) {
            return Some(done);
        }
        if let Some(done) = self.caret_key(k, cwd) {
            return Some(done);
        }
        if matches!(k, ChatInput::Enter) {
            self.skill_command(cwd); // `/name` → `@skill:name`, then sent as usual
        }
        match k {
            ChatInput::Cancel if !self.input.is_empty() => self.input.clear(),
            ChatInput::Cancel if self.is_busy() && self.connected => self.interrupt(),
            ChatInput::Cancel => {}
            ChatInput::EndOfInput if self.input.is_empty() && !self.is_busy() => {
                return Some(Some(ChatAction::Close));
            }
            ChatInput::EndOfInput => {}
            ChatInput::Enter if self.before_caret() == Some('\\') => {
                let mut chars: Vec<char> = self.input.chars().collect();
                let at = crate::chatcursor::caret_at(&self.input, self.caret_back);
                chars[at - 1] = '\n';
                self.input = chars.into_iter().collect();
            }
            ChatInput::Char('?') if self.input.is_empty() => return Some(Some(ChatAction::Help)),
            _ => return None,
        }
        Some(None)
    }
}

#[cfg(test)]
#[path = "chatctrl_tests.rs"]
mod tests;
