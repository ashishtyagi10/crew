use std::path::{Path, PathBuf};

use ratatui::prelude::*;

use crate::components::markdown::render_markdown_with_bg;

use super::hex::hex_dump;

pub struct ViewerState {
    /// Path to the file being viewed
    pub file_path: PathBuf,
    /// File contents split into lines
    pub lines: Vec<String>,
    /// Current scroll offset (top visible line)
    pub scroll_offset: usize,
    /// Whether the viewer is active (should be rendered)
    pub active: bool,
    /// Whether to wrap long lines
    pub wrap: bool,
    /// Hex view mode
    pub hex_mode: bool,
    /// Markdown preview mode
    pub markdown_mode: bool,
    /// Pre-rendered markdown lines
    pub markdown_lines: Vec<Line<'static>>,
    /// Search query
    pub search: Option<String>,
    /// Total number of lines
    pub total_lines: usize,
    /// Visible height from last render (for scroll clamping)
    pub visible_height: usize,
    /// File size in bytes
    pub file_size: u64,
    /// Follow/tail mode: auto-scroll to end and reload on tick
    pub follow: bool,
    /// Go-to-line input mode
    pub goto_input: Option<String>,
    /// Search input mode
    pub search_input: Option<String>,
}

const MAX_VIEW_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

impl ViewerState {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();

        if file_size > MAX_VIEW_SIZE {
            anyhow::bail!(
                "File too large ({:.1} MB). Max: 100 MB",
                file_size as f64 / 1_048_576.0
            );
        }

        // Read file contents - handle binary files gracefully
        let contents = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => {
                // Binary file - show hex dump
                let bytes = std::fs::read(path)?;
                return Ok(Self {
                    file_path: path.to_path_buf(),
                    lines: hex_dump(&bytes),
                    scroll_offset: 0,
                    active: true,
                    wrap: false,
                    hex_mode: true,
                    markdown_mode: false,
                    markdown_lines: Vec::new(),
                    search: None,
                    total_lines: bytes.len().div_ceil(16),
                    visible_height: 0,
                    file_size,
                    follow: false,
                    goto_input: None,
                    search_input: None,
                });
            }
        };

        let lines: Vec<String> = contents.lines().map(String::from).collect();
        let total_lines = lines.len();

        // Detect markdown files and pre-render
        let ext = path.extension().and_then(|e| e.to_str());
        let markdown_mode = matches!(ext, Some("md" | "markdown" | "mdx"));
        let markdown_lines = if markdown_mode {
            render_markdown_with_bg(&contents, Color::Rgb(22, 22, 26))
        } else {
            Vec::new()
        };
        let effective_total = if markdown_mode {
            markdown_lines.len()
        } else {
            total_lines
        };

        Ok(Self {
            file_path: path.to_path_buf(),
            lines,
            scroll_offset: 0,
            active: true,
            wrap: true,
            hex_mode: false,
            markdown_mode,
            markdown_lines,
            search: None,
            total_lines: effective_total,
            visible_height: 0,
            file_size,
            follow: false,
            goto_input: None,
            search_input: None,
        })
    }

    pub(super) fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub(super) fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }
}
