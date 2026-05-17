//! Inline text-entry routes: filter-mode pattern editing, and command-line
//! input handling (including slash suggestion navigation).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use farx_core::Action;

use super::super::App;

impl App {
    pub(super) fn key_route_filter(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.filter_active {
            return None;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.filter_active = false;
                self.filter_pattern.clear();
                self.active_tree().filter.clear();
                self.active_tree().rebuild();
            }
            (KeyCode::Enter, _) => {
                self.filter_active = false;
            }
            (KeyCode::Backspace, _) => {
                self.filter_pattern.pop();
                self.active_tree().filter = self.filter_pattern.clone();
                self.active_tree().rebuild();
            }
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.filter_pattern.push(ch);
                self.active_tree().filter = self.filter_pattern.clone();
                self.active_tree().rebuild();
            }
            (KeyCode::Up, _) => self.active_tree().move_cursor(-1),
            (KeyCode::Down, _) => self.active_tree().move_cursor(1),
            _ => {}
        }
        Some(Action::Noop)
    }

    pub(super) fn key_route_command_line(&mut self, key: KeyEvent) -> Option<Action> {
        if self.command_line.input.is_empty() {
            return None;
        }

        if self.slash_suggestions.is_some() {
            if let Some(action) = self.handle_slash_suggestion_key(key) {
                return Some(action);
            }
        }

        if key.code == KeyCode::Tab && self.command_line.suggestion.is_some() {
            self.command_line.accept_suggestion();
            self.command_line.last_typed_tick = self.tick_count;
            return Some(Action::Noop);
        }
        match (key.code, key.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) => Some(Action::CommandLineHistoryUp),
            (KeyCode::Down, KeyModifiers::NONE) => Some(Action::CommandLineHistoryDown),
            (KeyCode::Esc, _) => Some(Action::CommandLineClear),
            (KeyCode::Char(' '), KeyModifiers::NONE) => Some(Action::CommandLineInput(' ')),
            (KeyCode::Left, KeyModifiers::NONE) => {
                self.command_line.cursor_pos = self.command_line.cursor_pos.saturating_sub(1);
                Some(Action::Noop)
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                self.command_line.cursor_pos =
                    (self.command_line.cursor_pos + 1).min(self.command_line.input.len());
                Some(Action::Noop)
            }
            _ => None,
        }
    }

    /// Slash-suggestion popup key handling. Returns `Some(action)` when the
    /// key is consumed by the popup, `None` to fall through to normal
    /// command-line editing.
    fn handle_slash_suggestion_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up => {
                if let Some(ref mut ss) = self.slash_suggestions {
                    ss.move_up();
                }
                Some(Action::Noop)
            }
            KeyCode::Down => {
                if let Some(ref mut ss) = self.slash_suggestions {
                    ss.move_down();
                }
                Some(Action::Noop)
            }
            KeyCode::Tab => {
                if let Some(ref ss) = self.slash_suggestions {
                    if let Some(cmd) = ss.selected_command() {
                        self.command_line.input = cmd.to_string();
                        self.command_line.cursor_pos = self.command_line.input.len();
                        self.command_line.input.push(' ');
                        self.command_line.cursor_pos += 1;
                    }
                }
                self.slash_suggestions = None;
                Some(Action::Noop)
            }
            KeyCode::Enter => {
                if let Some(ref ss) = self.slash_suggestions {
                    if let Some(cmd) = ss.selected_command() {
                        self.command_line.input = cmd.to_string();
                        self.command_line.cursor_pos = self.command_line.input.len();
                    }
                }
                self.slash_suggestions = None;
                self.smart_execute_command();
                Some(Action::Noop)
            }
            KeyCode::Esc => {
                self.slash_suggestions = None;
                self.command_line.clear();
                Some(Action::Noop)
            }
            _ => None,
        }
    }
}
