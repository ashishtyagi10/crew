use crossterm::event::{KeyCode, KeyEvent};

use super::diff::{compute_diff, DiffLine};

#[derive(Debug, Clone, PartialEq)]
pub enum DiffAction {
    None,
    Close,
}

pub struct DiffViewState {
    pub left_path: std::path::PathBuf,
    pub right_path: std::path::PathBuf,
    pub(super) diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub active: bool,
}

impl DiffViewState {
    pub fn new(
        left_path: std::path::PathBuf,
        right_path: std::path::PathBuf,
    ) -> anyhow::Result<Self> {
        let left_content = std::fs::read_to_string(&left_path)?;
        let right_content = std::fs::read_to_string(&right_path)?;

        let left_lines: Vec<&str> = left_content.lines().collect();
        let right_lines: Vec<&str> = right_content.lines().collect();

        let diff_lines = compute_diff(&left_lines, &right_lines);

        Ok(Self {
            left_path,
            right_path,
            diff_lines,
            scroll_offset: 0,
            active: true,
        })
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> DiffAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DiffAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                DiffAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll_offset + 1 < self.diff_lines.len() {
                    self.scroll_offset += 1;
                }
                DiffAction::None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                DiffAction::None
            }
            KeyCode::PageDown => {
                self.scroll_offset =
                    (self.scroll_offset + 20).min(self.diff_lines.len().saturating_sub(1));
                DiffAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                DiffAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.diff_lines.len().saturating_sub(1);
                DiffAction::None
            }
            _ => DiffAction::None,
        }
    }
}
