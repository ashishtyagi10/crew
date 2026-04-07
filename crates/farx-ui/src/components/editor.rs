use crate::components::syntax::{highlight_line, Language};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum EditorAction {
    None,
    Close,
    SaveAndClose,
}

#[derive(Debug, Clone)]
struct UndoEntry {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum EditorMode {
    Normal,
    Search,
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
    mode: EditorMode,
    search_query: String,
    search_cursor: usize,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    last_save_undo_len: usize,
}

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

impl EditorState {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let meta = std::fs::metadata(path)?;
            if meta.len() > MAX_FILE_SIZE {
                anyhow::bail!(
                    "File too large ({:.1} MB). Max: 100 MB",
                    meta.len() as f64 / 1_048_576.0
                );
            }
        }
        let contents = if path.exists() {
            std::fs::read_to_string(path)?
        } else {
            String::new()
        };
        let lines: Vec<String> = if contents.is_empty() {
            vec![String::new()]
        } else {
            contents.lines().map(String::from).collect()
        };

        Ok(Self {
            file_path: path.to_path_buf(),
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            horizontal_scroll: 0,
            modified: false,
            active: true,
            mode: EditorMode::Normal,
            search_query: String::new(),
            search_cursor: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_save_undo_len: 0,
        })
    }

    fn save_undo(&mut self) {
        self.undo_stack.push(UndoEntry {
            lines: self.lines.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        });
        self.redo_stack.clear();
        // Limit undo history
        if self.undo_stack.len() > 1000 {
            self.undo_stack.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(UndoEntry {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = entry.lines;
            self.cursor_line = entry.cursor_line;
            self.cursor_col = entry.cursor_col;
            self.modified = self.undo_stack.len() != self.last_save_undo_len;
        }
    }

    fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(UndoEntry {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = entry.lines;
            self.cursor_line = entry.cursor_line;
            self.cursor_col = entry.cursor_col;
            self.modified = self.undo_stack.len() != self.last_save_undo_len;
        }
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let content = self.lines.join("\n");
        // Add trailing newline if file had content
        let content = if content.is_empty() {
            content
        } else {
            content + "\n"
        };
        std::fs::write(&self.file_path, &content)?;
        self.modified = false;
        self.last_save_undo_len = self.undo_stack.len();
        Ok(())
    }

    fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_line)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn current_line_len(&self) -> usize {
        self.current_line().len()
    }

    fn clamp_cursor_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
        // Ensure we're at a char boundary
        if self.cursor_line < self.lines.len() {
            while self.cursor_col > 0
                && !self.lines[self.cursor_line].is_char_boundary(self.cursor_col)
            {
                self.cursor_col -= 1;
            }
        }
    }

    fn insert_char(&mut self, ch: char) {
        self.save_undo();
        if self.cursor_line < self.lines.len() {
            self.lines[self.cursor_line].insert(self.cursor_col, ch);
            self.cursor_col += ch.len_utf8();
        }
        self.modified = true;
    }

    fn insert_newline(&mut self) {
        self.save_undo();
        if self.cursor_line < self.lines.len() {
            // Ensure cursor_col is at a char boundary
            let line = &self.lines[self.cursor_line];
            let col = self.cursor_col.min(line.len());
            let col = if col > 0 && !line.is_char_boundary(col) {
                line.char_indices()
                    .rev()
                    .find(|&(i, _)| i <= col)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            } else {
                col
            };
            let rest = self.lines[self.cursor_line][col..].to_string();
            self.lines[self.cursor_line].truncate(col);
            self.cursor_line += 1;
            self.lines.insert(self.cursor_line, rest);
            self.cursor_col = 0;
        }
        self.modified = true;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.save_undo();
            self.cursor_col -= 1;
            // Handle multi-byte chars
            while self.cursor_col > 0
                && !self.lines[self.cursor_line].is_char_boundary(self.cursor_col)
            {
                self.cursor_col -= 1;
            }
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line > 0 {
            self.save_undo();
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current);
            self.modified = true;
        }
    }

    fn delete_char(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor_col < line_len {
            self.save_undo();
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.save_undo();
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
            self.modified = true;
        }
    }

    fn find_next(&mut self) {
        if self.search_query.is_empty() || self.lines.is_empty() {
            return;
        }
        let query = self.search_query.clone();
        // Search from current position forward
        let start_line = self.cursor_line.min(self.lines.len() - 1);
        let start_col = self.cursor_col + 1;

        for i in 0..self.lines.len() {
            let line_idx = (start_line + i) % self.lines.len();
            let search_from = if i == 0 {
                // Ensure search_from is at a char boundary
                let line = &self.lines[line_idx];
                let mut sf = start_col.min(line.len());
                while sf > 0 && sf < line.len() && !line.is_char_boundary(sf) {
                    sf += 1;
                }
                sf
            } else {
                0
            };
            if search_from <= self.lines[line_idx].len() {
                if let Some(pos) = self.lines[line_idx][search_from..].find(&query) {
                    self.cursor_line = line_idx;
                    self.cursor_col = search_from + pos;
                    return;
                }
            }
        }
    }

    pub fn scroll_to_cursor(&mut self, visible_height: usize, visible_width: usize) {
        // Vertical scroll
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
        if self.cursor_line >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_line - visible_height + 1;
        }
        // Horizontal scroll
        let gutter_width = 6; // line number width
        let text_width = visible_width.saturating_sub(gutter_width);
        if self.cursor_col < self.horizontal_scroll {
            self.horizontal_scroll = self.cursor_col;
        }
        if self.cursor_col >= self.horizontal_scroll + text_width {
            self.horizontal_scroll = self.cursor_col - text_width + 1;
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> EditorAction {
        match self.mode {
            EditorMode::ConfirmExit => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        self.active = false;
                        return EditorAction::Close;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.mode = EditorMode::Normal;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if self.save().is_ok() {
                            self.active = false;
                            return EditorAction::SaveAndClose;
                        }
                        self.mode = EditorMode::Normal;
                    }
                    _ => {}
                }
                return EditorAction::None;
            }
            EditorMode::Search => {
                match key.code {
                    KeyCode::Enter => {
                        self.mode = EditorMode::Normal;
                        self.find_next();
                    }
                    KeyCode::Esc => {
                        self.mode = EditorMode::Normal;
                    }
                    KeyCode::Char(ch) => {
                        self.search_query.insert(self.search_cursor, ch);
                        self.search_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if self.search_cursor > 0 {
                            self.search_cursor -= 1;
                            self.search_query.remove(self.search_cursor);
                        }
                    }
                    KeyCode::Left => {
                        self.search_cursor = self.search_cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        self.search_cursor = (self.search_cursor + 1).min(self.search_query.len());
                    }
                    _ => {}
                }
                return EditorAction::None;
            }
            EditorMode::Normal => {}
        }

        // Normal mode key handling
        match (key.code, key.modifiers) {
            // Exit
            (KeyCode::Esc, _) | (KeyCode::F(10), _) => {
                if self.modified {
                    self.mode = EditorMode::ConfirmExit;
                } else {
                    self.active = false;
                    return EditorAction::Close;
                }
            }
            // Save
            (KeyCode::F(2), KeyModifiers::NONE) => {
                let _ = self.save();
            }
            (KeyCode::F(2), KeyModifiers::SHIFT) => {
                if self.save().is_ok() {
                    self.active = false;
                    return EditorAction::SaveAndClose;
                }
            }
            // Search
            (KeyCode::F(7), KeyModifiers::NONE) => {
                self.mode = EditorMode::Search;
                self.search_cursor = self.search_query.len();
            }
            // Find next
            (KeyCode::F(3), KeyModifiers::NONE) | (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.find_next();
            }
            // Undo/Redo
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                self.undo();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.redo();
            }
            // Navigation
            (KeyCode::Up, KeyModifiers::NONE) => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_col();
                }
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_cursor_col();
                }
            }
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    // Skip back to char boundary for multi-byte chars
                    let line = &self.lines[self.cursor_line];
                    while self.cursor_col > 0 && !line.is_char_boundary(self.cursor_col) {
                        self.cursor_col -= 1;
                    }
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.current_line_len();
                }
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                if self.cursor_col < self.current_line_len() {
                    self.cursor_col += 1;
                    // Skip forward to char boundary for multi-byte chars
                    let line = &self.lines[self.cursor_line];
                    while self.cursor_col < line.len() && !line.is_char_boundary(self.cursor_col) {
                        self.cursor_col += 1;
                    }
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.cursor_col = 0;
            }
            (KeyCode::End, KeyModifiers::NONE) => {
                self.cursor_col = self.current_line_len();
            }
            (KeyCode::Home, KeyModifiers::CONTROL) => {
                self.cursor_line = 0;
                self.cursor_col = 0;
            }
            (KeyCode::End, KeyModifiers::CONTROL) => {
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.current_line_len();
            }
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.cursor_line = self.cursor_line.saturating_sub(30);
                self.clamp_cursor_col();
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.cursor_line = (self.cursor_line + 30).min(self.lines.len().saturating_sub(1));
                self.clamp_cursor_col();
            }
            // Editing
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.insert_char(ch);
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.insert_newline();
            }
            (KeyCode::Backspace, _) => {
                self.backspace();
            }
            (KeyCode::Delete, _) => {
                self.delete_char();
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                // Insert 4 spaces
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            _ => {}
        }
        EditorAction::None
    }
}

pub fn render_editor(frame: &mut Frame, state: &EditorState, _theme: &Theme) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let file_name = state
        .file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "new file".to_string());
    let modified_marker = if state.modified { " [modified]" } else { "" };
    let title = format!(" Edit: {}{} ", file_name, modified_marker);

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

    let visible_height = inner.height.saturating_sub(1) as usize; // -1 for status bar
    let gutter_width = 6u16;
    let text_width = inner.width.saturating_sub(gutter_width) as usize;

    // Detect language from file extension
    let ext = state.file_path.extension().and_then(|e| e.to_str());
    let lang = Language::from_extension(ext);

    // Build visible lines with syntax highlighting
    let mut text_lines: Vec<Line> = Vec::new();
    for i in state.scroll_offset..(state.scroll_offset + visible_height).min(state.lines.len()) {
        let line_num = format!("{:>5} ", i + 1);
        let line = &state.lines[i];

        // Apply horizontal scroll
        let visible_text: String = line
            .chars()
            .skip(state.horizontal_scroll)
            .take(text_width)
            .collect();

        let is_cursor_line = i == state.cursor_line;
        let bg = if is_cursor_line {
            Color::Indexed(236)
        } else {
            Color::Rgb(22, 22, 26)
        };
        let line_num_style = Style::default().fg(Color::DarkGray).bg(bg);

        let mut spans = vec![Span::styled(line_num, line_num_style)];

        // Syntax highlight the visible portion
        let highlighted = highlight_line(&visible_text, lang, bg);
        if highlighted.is_empty() {
            spans.push(Span::styled(
                format!("{:<width$}", visible_text, width = text_width),
                Style::default().fg(Color::Rgb(200, 200, 210)).bg(bg),
            ));
        } else {
            spans.extend(highlighted);
            // Pad remaining width with background
            let used: usize = visible_text.len();
            if used < text_width {
                spans.push(Span::styled(
                    " ".repeat(text_width - used),
                    Style::default().bg(bg),
                ));
            }
        }

        text_lines.push(Line::from(spans));
    }

    // Fill remaining lines
    for _ in text_lines.len()..visible_height {
        text_lines.push(Line::from(vec![
            Span::styled(
                "    ~ ",
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(Color::Rgb(22, 22, 26)),
            ),
            Span::styled(
                " ".repeat(text_width),
                Style::default().bg(Color::Rgb(22, 22, 26)),
            ),
        ]));
    }

    let text_area = Rect::new(inner.x, inner.y, inner.width, visible_height as u16);
    frame.render_widget(Paragraph::new(text_lines), text_area);

    // Scrollbar
    if state.lines.len() > visible_height {
        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        let mut scrollbar_state =
            ScrollbarState::new(state.lines.len().saturating_sub(visible_height))
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

    // Status bar at bottom
    let status_y = inner.y + inner.height.saturating_sub(1);
    let status = match state.mode {
        EditorMode::ConfirmExit => {
            " File modified. Save? (Y)es / (N)o / (S)ave and exit / (Esc) cancel ".to_string()
        }
        EditorMode::Search => {
            format!(" Search: {}_  (Enter=Find, Esc=Cancel)", state.search_query)
        }
        EditorMode::Normal => {
            format!(
                " Ln {}, Col {} | {} | F2=Save  F7=Search  F3=FindNext  Ctrl+Z=Undo  Esc=Exit ",
                state.cursor_line + 1,
                state.cursor_col + 1,
                if state.modified { "Modified" } else { "Saved" },
            )
        }
    };
    let status_line = Line::from(Span::styled(
        format!("{:<width$}", status, width = inner.width as usize),
        Style::default()
            .fg(Color::Rgb(16, 16, 18))
            .bg(Color::Rgb(220, 170, 60)),
    ));
    frame.render_widget(
        Paragraph::new(status_line),
        Rect::new(inner.x, status_y, inner.width, 1),
    );

    // Position the cursor
    if state.mode == EditorMode::Search {
        let cursor_x = inner.x + 9 + state.search_cursor as u16; // " Search: " is 9 chars
        frame.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), status_y));
    } else if state.mode == EditorMode::Normal {
        let visual_col = state.cursor_col.saturating_sub(state.horizontal_scroll) as u16;
        let cursor_x = inner.x + gutter_width + visual_col;
        let cursor_y = inner.y + (state.cursor_line.saturating_sub(state.scroll_offset)) as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + visible_height as u16 {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
