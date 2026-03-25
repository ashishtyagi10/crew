use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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
    /// Search query
    pub search: Option<String>,
    /// Total number of lines
    pub total_lines: usize,
    /// File size in bytes
    pub file_size: u64,
}

impl ViewerState {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();

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
                    search: None,
                    total_lines: bytes.len().div_ceil(16),
                    file_size,
                });
            }
        };

        let lines: Vec<String> = contents.lines().map(String::from).collect();
        let total_lines = lines.len();

        Ok(Self {
            file_path: path.to_path_buf(),
            lines,
            scroll_offset: 0,
            active: true,
            wrap: false,
            hex_mode: false,
            search: None,
            total_lines,
            file_size,
        })
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ViewerAction {
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
                self.scroll_offset = self.total_lines.saturating_sub(1);
                ViewerAction::None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.wrap = !self.wrap;
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = (self.scroll_offset + amount).min(self.total_lines.saturating_sub(1));
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

pub fn render_viewer(frame: &mut Frame, state: &ViewerState, _theme: &Theme) {
    let area = frame.area();

    // Title with file info
    let file_name = state
        .file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mode_str = if state.hex_mode { " [HEX]" } else { "" };
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

    let visible_height = inner.height as usize;

    // Detect language from file extension
    let ext = state.file_path.extension().and_then(|e| e.to_str());
    let lang = Language::from_extension(ext);
    let bg = Color::Rgb(22, 22, 26);

    // Build visible lines with syntax highlighting
    let mut text_lines: Vec<Line> = Vec::new();
    for i in state.scroll_offset..(state.scroll_offset + visible_height).min(state.total_lines) {
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

            text_lines.push(Line::from(spans));
        }
    }

    let paragraph = if state.wrap {
        Paragraph::new(text_lines).wrap(Wrap { trim: false })
    } else {
        Paragraph::new(text_lines)
    };

    frame.render_widget(paragraph, inner);

    // Status bar at the bottom of the viewer area
    let status_y = area.y + area.height.saturating_sub(1);
    let percentage = if state.total_lines == 0 {
        100
    } else {
        ((state.scroll_offset + visible_height).min(state.total_lines) * 100) / state.total_lines
    };
    let status_text = format!(
        " Line {}/{} ({}%) | {}Esc/F3/q=Close  PgUp/PgDn=Scroll  Ctrl+W=Wrap ",
        state.scroll_offset + 1,
        state.total_lines,
        percentage,
        if state.wrap { "Wrap:ON  " } else { "" },
    );
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
