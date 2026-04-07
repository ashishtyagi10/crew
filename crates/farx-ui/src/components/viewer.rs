use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::components::markdown::render_markdown_with_bg;
use crate::components::syntax::{highlight_line, Language};
use crate::theme::Theme;

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
            wrap: false,
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

    fn find_in_viewer(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }
        let query_lower = query.to_lowercase();
        // Search from current scroll position forward
        for i in self.scroll_offset..self.total_lines {
            if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                self.scroll_offset = i;
                self.search = Some(query.to_string());
                return;
            }
        }
        // Wrap around from beginning
        for i in 0..self.scroll_offset {
            if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                self.scroll_offset = i;
                self.search = Some(query.to_string());
                return;
            }
        }
    }

    fn find_next_in_viewer(&mut self) {
        if let Some(query) = self.search.clone() {
            let query_lower = query.to_lowercase();
            for i in (self.scroll_offset + 1)..self.total_lines {
                if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                    self.scroll_offset = i;
                    return;
                }
            }
            // Wrap
            for i in 0..=self.scroll_offset {
                if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                    self.scroll_offset = i;
                    return;
                }
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ViewerAction {
        // Search input mode
        if let Some(ref mut input) = self.search_input {
            match key.code {
                KeyCode::Enter => {
                    let query = input.clone();
                    self.search_input = None;
                    self.find_in_viewer(&query);
                }
                KeyCode::Esc => {
                    self.search_input = None;
                }
                KeyCode::Char(ch) => {
                    input.push(ch);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            }
            return ViewerAction::None;
        }

        // Go-to-line input mode
        if let Some(ref mut input) = self.goto_input {
            match key.code {
                KeyCode::Enter => {
                    if let Ok(line_num) = input.parse::<usize>() {
                        self.scroll_offset = line_num
                            .saturating_sub(1)
                            .min(self.total_lines.saturating_sub(1));
                    }
                    self.goto_input = None;
                }
                KeyCode::Esc => {
                    self.goto_input = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    input.push(ch);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            }
            return ViewerAction::None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::F(10) | KeyCode::Char('q') => {
                self.active = false;
                ViewerAction::Close
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up(1);
                ViewerAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down(1);
                ViewerAction::None
            }
            KeyCode::PageUp => {
                self.scroll_up(30);
                ViewerAction::None
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll_down(30);
                ViewerAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                ViewerAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
                ViewerAction::None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.wrap = !self.wrap;
                ViewerAction::None
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.goto_input = Some(String::new());
                ViewerAction::None
            }
            KeyCode::Char('/') | KeyCode::F(7) => {
                self.search_input = Some(String::new());
                ViewerAction::None
            }
            KeyCode::Char('n') => {
                self.find_next_in_viewer();
                ViewerAction::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.follow = !self.follow;
                if self.follow {
                    // Jump to end when enabling follow
                    self.scroll_offset =
                        self.total_lines.saturating_sub(self.visible_height.max(1));
                }
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }

    /// Reload file contents (for follow/tail mode). Returns true if content changed.
    pub fn reload_if_follow(&mut self) -> bool {
        if !self.follow || self.hex_mode {
            return false;
        }
        let Ok(contents) = std::fs::read_to_string(&self.file_path) else {
            return false;
        };
        let new_lines: Vec<String> = contents.lines().map(String::from).collect();
        if new_lines.len() == self.lines.len() {
            return false;
        }
        self.lines = new_lines;
        self.total_lines = self.lines.len();
        self.scroll_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
        true
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> ViewerAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3);
                ViewerAction::None
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3);
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewerAction {
    None,
    Close,
}

fn hex_dump(bytes: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let offset = i * 16;
        let hex: String = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        lines.push(format!("{:08X}  {:<48}  {}", offset, hex, ascii));
    }
    lines
}

pub fn render_viewer(frame: &mut Frame, state: &mut ViewerState, _theme: &Theme) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    // Title with file info
    let file_name = state
        .file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mode_str = if state.hex_mode {
        " [HEX]"
    } else if state.markdown_mode {
        " [MD]"
    } else {
        ""
    };
    let title = format!(
        " {} - {}{} ",
        file_name,
        format_file_size(state.file_size),
        mode_str
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Rgb(200, 200, 210))
                .bg(Color::Rgb(22, 22, 26)),
        )
        .style(
            Style::default()
                .bg(Color::Rgb(22, 22, 26))
                .fg(Color::Rgb(200, 200, 210)),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve 1 row for the status bar (rendered over bottom border)
    let content_height = inner.height as usize;
    state.visible_height = content_height;

    // Re-clamp scroll offset after visible_height is known
    if state.total_lines > content_height {
        state.scroll_offset = state
            .scroll_offset
            .min(state.total_lines.saturating_sub(content_height));
    } else {
        state.scroll_offset = 0;
    }

    let bg = Color::Rgb(22, 22, 26);

    let text_lines: Vec<Line> = if state.markdown_mode {
        // Markdown preview: use pre-rendered lines
        let end = (state.scroll_offset + content_height).min(state.markdown_lines.len());
        state.markdown_lines[state.scroll_offset..end].to_vec()
    } else {
        // Detect language from file extension
        let ext = state.file_path.extension().and_then(|e| e.to_str());
        let lang = Language::from_extension(ext);

        let mut lines: Vec<Line> = Vec::new();
        for i in state.scroll_offset..(state.scroll_offset + content_height).min(state.total_lines)
        {
            if i < state.lines.len() {
                let line_num = format!("{:>6} ", i + 1);
                let content = &state.lines[i];

                let mut spans = vec![Span::styled(
                    line_num,
                    Style::default().fg(Color::DarkGray).bg(bg),
                )];

                if state.hex_mode {
                    spans.push(Span::styled(
                        content.as_str(),
                        Style::default().fg(Color::Rgb(200, 200, 210)).bg(bg),
                    ));
                } else {
                    spans.extend(highlight_line(content, lang, bg));
                }

                lines.push(Line::from(spans));
            }
        }
        lines
    };

    let paragraph = if state.wrap {
        Paragraph::new(text_lines).wrap(Wrap { trim: false })
    } else {
        Paragraph::new(text_lines)
    };

    frame.render_widget(paragraph, inner);

    // Scrollbar
    if state.total_lines > content_height {
        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        let mut scrollbar_state =
            ScrollbarState::new(state.total_lines.saturating_sub(content_height))
                .position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(Color::Rgb(50, 50, 55)))
                .thumb_style(Style::default().fg(Color::Rgb(120, 120, 140))),
            scrollbar_area,
            &mut scrollbar_state,
        );
    }

    // Status bar at the bottom of the viewer area
    let status_y = area.y + area.height.saturating_sub(1);
    let percentage = if state.total_lines == 0 {
        100
    } else {
        ((state.scroll_offset + content_height).min(state.total_lines) * 100) / state.total_lines
    };
    let follow_indicator = if state.follow { "FOLLOW  " } else { "" };
    let status_text = if let Some(ref input) = state.search_input {
        format!(" Search: {}_ (Enter=Find, Esc=Cancel) ", input)
    } else if let Some(ref input) = state.goto_input {
        format!(" Go to line: {}_ (Enter=Go, Esc=Cancel) ", input)
    } else {
        format!(
            " Line {}/{} ({}%) | {}{}Esc/F3/q=Close  PgUp/PgDn  Ctrl+G=GoTo  Ctrl+F=Follow ",
            state.scroll_offset + 1,
            state.total_lines,
            percentage,
            if state.wrap { "Wrap:ON  " } else { "" },
            follow_indicator,
        )
    };
    let status_line = Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(Color::Rgb(16, 16, 18))
            .bg(Color::Rgb(220, 170, 60)),
    ));
    // Render over the bottom border
    frame.render_widget(
        Paragraph::new(status_line),
        Rect::new(area.x, status_y, area.width, 1),
    );
}

fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
