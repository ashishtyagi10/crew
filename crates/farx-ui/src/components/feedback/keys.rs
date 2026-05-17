//! Key handling for the feedback system.

use crossterm::event::{KeyCode, KeyEvent};

use super::state::FeedbackState;
use super::types::FeedbackResult;

impl FeedbackState {
    /// Handle a key event. Returns whether it was consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> FeedbackResult {
        // Confirmation takes priority
        if self.confirm.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    return FeedbackResult::Confirmed(0);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirm = None;
                    return FeedbackResult::Rejected;
                }
                _ => return FeedbackResult::Consumed,
            }
        }

        // Output panel scroll
        if self.output_visible {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    self.output_visible = false;
                    self.output_lines.clear();
                    return FeedbackResult::Consumed;
                }
                KeyCode::Up => {
                    self.output_scroll = self.output_scroll.saturating_sub(1);
                    return FeedbackResult::Consumed;
                }
                KeyCode::Down => {
                    if self.output_scroll + 1 < self.output_lines.len() {
                        self.output_scroll += 1;
                    }
                    return FeedbackResult::Consumed;
                }
                KeyCode::PageUp => {
                    self.output_scroll = self.output_scroll.saturating_sub(20);
                    return FeedbackResult::Consumed;
                }
                KeyCode::PageDown => {
                    self.output_scroll =
                        (self.output_scroll + 20).min(self.output_lines.len().saturating_sub(1));
                    return FeedbackResult::Consumed;
                }
                _ => {
                    // Any other key dismisses the output
                    self.output_visible = false;
                    self.output_lines.clear();
                    return FeedbackResult::Consumed;
                }
            }
        }

        // Any keypress clears stale messages
        if !self.messages.is_empty() {
            self.messages.clear();
        }

        FeedbackResult::NotHandled
    }
}
