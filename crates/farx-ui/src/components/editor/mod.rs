use ratatui::prelude::Line;
use std::path::PathBuf;

mod edit;
mod io;
mod keys;
mod keys_nav;
mod render;
mod render_nowrap;
mod render_preview;
mod render_wrap;
mod scroll;
mod search;
mod undo;

pub use render::render_editor;

#[derive(Debug, Clone, PartialEq)]
pub enum EditorAction {
    None,
    Close,
    SaveAndClose,
}

#[derive(Debug, Clone)]
pub(super) struct UndoEntry {
    pub(super) lines: Vec<String>,
    pub(super) cursor_line: usize,
    pub(super) cursor_col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum EditorMode {
    Normal,
    Search,
    GotoLine,
    ConfirmExit,
}

pub struct EditorState {
    pub file_path: PathBuf,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
    pub modified: bool,
    pub active: bool,
    /// Word wrap mode (visual only — data stays unwrapped)
    pub wrap: bool,
    /// Markdown preview mode (read-only rendered view)
    pub preview_mode: bool,
    /// Pre-rendered markdown lines for preview
    pub(super) preview_lines: Vec<Line<'static>>,
    /// Scroll offset for preview mode
    pub(super) preview_scroll: usize,
    pub(super) mode: EditorMode,
    pub(super) search_query: String,
    pub(super) search_cursor: usize,
    pub(super) goto_line_input: String,
    pub(super) undo_stack: Vec<UndoEntry>,
    pub(super) redo_stack: Vec<UndoEntry>,
    pub(super) last_save_undo_len: usize,
}

pub(super) const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
