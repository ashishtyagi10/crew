use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

use super::storage::Bookmark;

#[derive(Debug, Clone, PartialEq)]
pub enum BookmarkAction {
    None,
    Close,
    GoTo(PathBuf),
    Delete(usize),
}

pub struct BookmarkState {
    pub active: bool,
    pub bookmarks: Vec<Bookmark>,
    pub cursor: usize,
    pub scroll: usize,
}

impl BookmarkState {
    pub fn new(bookmarks: Vec<Bookmark>) -> Self {
        Self {
            active: true,
            bookmarks,
            cursor: 0,
            scroll: 0,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> BookmarkAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                BookmarkAction::Close
            }
            KeyCode::Enter => {
                if let Some(bm) = self.bookmarks.get(self.cursor) {
                    let path = bm.path.clone();
                    self.active = false;
                    BookmarkAction::GoTo(path)
                } else {
                    BookmarkAction::None
                }
            }
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    if self.cursor < self.scroll {
                        self.scroll = self.cursor;
                    }
                }
                BookmarkAction::None
            }
            KeyCode::Down => {
                if self.cursor + 1 < self.bookmarks.len() {
                    self.cursor += 1;
                }
                BookmarkAction::None
            }
            KeyCode::Delete | KeyCode::F(8) => {
                if !self.bookmarks.is_empty() {
                    let idx = self.cursor;
                    self.bookmarks.remove(idx);
                    if self.cursor >= self.bookmarks.len() && self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    BookmarkAction::Delete(idx)
                } else {
                    BookmarkAction::None
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
                self.scroll = 0;
                BookmarkAction::None
            }
            KeyCode::End => {
                if !self.bookmarks.is_empty() {
                    self.cursor = self.bookmarks.len() - 1;
                }
                BookmarkAction::None
            }
            _ => BookmarkAction::None,
        }
    }
}
