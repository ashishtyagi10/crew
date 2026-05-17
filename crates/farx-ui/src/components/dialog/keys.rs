use crossterm::event::{KeyCode, KeyEvent};

use super::state::{DialogKind, DialogResult, DialogState};

pub(super) fn handle_key_event(state: &mut DialogState, key: KeyEvent) {
    match &mut state.kind {
        DialogKind::Input {
            input, cursor_pos, ..
        } => match key.code {
            KeyCode::Enter => {
                state.result = DialogResult::Confirm(Some(input.clone()));
            }
            KeyCode::Esc => {
                state.result = DialogResult::Cancel;
            }
            KeyCode::Char(ch) => {
                input.insert(*cursor_pos, ch);
                *cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    input.remove(*cursor_pos);
                }
            }
            KeyCode::Delete => {
                if *cursor_pos < input.len() {
                    input.remove(*cursor_pos);
                }
            }
            KeyCode::Left => {
                *cursor_pos = cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                *cursor_pos = (*cursor_pos + 1).min(input.len());
            }
            KeyCode::Home => {
                *cursor_pos = 0;
            }
            KeyCode::End => {
                *cursor_pos = input.len();
            }
            _ => {}
        },
        DialogKind::Confirm { .. } => match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                state.result = DialogResult::Confirm(None);
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.result = DialogResult::Cancel;
            }
            _ => {}
        },
        DialogKind::Message { .. } | DialogKind::Error { .. } => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                state.result = DialogResult::Cancel;
            }
            _ => {}
        },
    }
}
