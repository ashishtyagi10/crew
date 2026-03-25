use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    None,
    Close,
    /// Navigate to the selected result
    GoTo(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
enum SearchField {
    Pattern,
    Content,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct SearchState {
    pub active: bool,
    field: SearchField,
    pub pattern: String,
    pub content_query: String,
    pattern_cursor: usize,
    content_cursor: usize,
    pub results: Vec<SearchResult>,
    pub result_cursor: usize,
    pub result_scroll: usize,
    pub searching: bool,
    pub search_dir: PathBuf,
}

impl SearchState {
    pub fn new(search_dir: PathBuf) -> Self {
        Self {
            active: true,
            field: SearchField::Pattern,
            pattern: "*".to_string(),
            content_query: String::new(),
            pattern_cursor: 1,
            content_cursor: 0,
            results: Vec::new(),
            result_cursor: 0,
            result_scroll: 0,
            searching: false,
            search_dir,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> SearchAction {
        // If we have results and are browsing them
        if !self.results.is_empty() && self.field == SearchField::Pattern {
            match key.code {
                KeyCode::Esc => {
                    self.active = false;
                    return SearchAction::Close;
                }
                KeyCode::Enter => {
                    if !self.results.is_empty() {
                        let result = &self.results[self.result_cursor];
                        let path = if result.is_dir {
                            result.path.clone()
                        } else {
                            result.path.parent().unwrap_or(Path::new("/")).to_path_buf()
                        };
                        self.active = false;
                        return SearchAction::GoTo(path);
                    }
                }
                KeyCode::Up => {
                    if self.result_cursor > 0 {
                        self.result_cursor -= 1;
                        if self.result_cursor < self.result_scroll {
                            self.result_scroll = self.result_cursor;
                        }
                    }
                    return SearchAction::None;
                }
                KeyCode::Down => {
                    if self.result_cursor + 1 < self.results.len() {
                        self.result_cursor += 1;
                    }
                    return SearchAction::None;
                }
                KeyCode::Tab => {
                    // Clear results and go back to editing
                    self.results.clear();
                    self.result_cursor = 0;
                    return SearchAction::None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.active = false;
                SearchAction::Close
            }
            KeyCode::Tab => {
                self.field = match self.field {
                    SearchField::Pattern => SearchField::Content,
                    SearchField::Content => SearchField::Pattern,
                };
                SearchAction::None
            }
            KeyCode::Enter => {
                // Start search
                self.execute_search();
                SearchAction::None
            }
            KeyCode::Char(ch) => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern.insert(self.pattern_cursor, ch);
                        self.pattern_cursor += 1;
                    }
                    SearchField::Content => {
                        self.content_query.insert(self.content_cursor, ch);
                        self.content_cursor += 1;
                    }
                }
                SearchAction::None
            }
            KeyCode::Backspace => {
                match self.field {
                    SearchField::Pattern => {
                        if self.pattern_cursor > 0 {
                            self.pattern_cursor -= 1;
                            self.pattern.remove(self.pattern_cursor);
                        }
                    }
                    SearchField::Content => {
                        if self.content_cursor > 0 {
                            self.content_cursor -= 1;
                            self.content_query.remove(self.content_cursor);
                        }
                    }
                }
                SearchAction::None
            }
            KeyCode::Left => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern_cursor = self.pattern_cursor.saturating_sub(1);
                    }
                    SearchField::Content => {
                        self.content_cursor = self.content_cursor.saturating_sub(1);
                    }
                }
                SearchAction::None
            }
            KeyCode::Right => {
                match self.field {
                    SearchField::Pattern => {
                        self.pattern_cursor = (self.pattern_cursor + 1).min(self.pattern.len());
                    }
                    SearchField::Content => {
                        self.content_cursor =
                            (self.content_cursor + 1).min(self.content_query.len());
                    }
                }
                SearchAction::None
            }
            _ => SearchAction::None,
        }
    }

    fn execute_search(&mut self) {
        self.results.clear();
        self.result_cursor = 0;
        self.result_scroll = 0;
        self.searching = true;

        let pattern = self.pattern.clone();
        let content_query = self.content_query.clone();
        let search_dir = self.search_dir.clone();

        // Synchronous search (could be made async later)
        search_recursive(
            &search_dir,
            &pattern,
            &content_query,
            &mut self.results,
            5000,
        );

        self.searching = false;
    }
}

fn matches_glob(name: &str, pattern: &str) -> bool {
    // Simple glob matching: * matches anything, ? matches single char
    let pattern = pattern.to_lowercase();
    let name = name.to_lowercase();
    glob_match(&name, &pattern)
}

fn glob_match(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let mut ti = 0;
    let mut pi = 0;
    let mut star_pi = None;
    let mut star_ti = 0;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }

    pi == pattern.len()
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    content_query: &str,
    results: &mut Vec<SearchResult>,
    limit: usize,
) {
    if results.len() >= limit {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        if results.len() >= limit {
            return;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();

        // Check name match
        if matches_glob(&name, pattern) {
            // If content query is set and it's a file, check content
            let content_match = if !content_query.is_empty() && !is_dir {
                match std::fs::read_to_string(&path) {
                    Ok(content) => content
                        .to_lowercase()
                        .contains(&content_query.to_lowercase()),
                    Err(_) => false,
                }
            } else {
                true // No content filter or is a directory
            };

            if content_match {
                results.push(SearchResult {
                    path: path.clone(),
                    name: name.clone(),
                    is_dir,
                    size: if is_dir { 0 } else { metadata.len() },
                });
            }
        }

        // Recurse into directories
        if is_dir && !name.starts_with('.') {
            search_recursive(&path, pattern, content_query, results, limit);
        }
    }
}

pub fn render_search(frame: &mut Frame, state: &SearchState, _theme: &Theme) {
    let area = frame.area();

    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = (area.height - 4).min(30);
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Find Files (Alt+F7) ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut y_offset = 0u16;

    // Search dir
    let dir_line = Line::from(vec![
        Span::styled(
            " Search in: ",
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
        ),
        Span::styled(
            state.search_dir.display().to_string(),
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(dir_line),
        Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
    );
    y_offset += 2;

    // Pattern field
    let pattern_active = state.field == SearchField::Pattern && state.results.is_empty();
    let pattern_label_style = Style::default()
        .fg(if pattern_active {
            Color::Yellow
        } else {
            Color::Cyan
        })
        .bg(Color::Indexed(236));
    let pattern_input_style = Style::default().fg(Color::White).bg(if pattern_active {
        Color::Indexed(238)
    } else {
        Color::Indexed(237)
    });

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(" File mask:", pattern_label_style))),
        Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
    );
    y_offset += 1;

    let pattern_display = format!(
        " {:<width$}",
        state.pattern,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            pattern_display,
            pattern_input_style,
        ))),
        Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
    );
    if pattern_active {
        frame.set_cursor_position((
            inner.x + 1 + state.pattern_cursor as u16,
            inner.y + y_offset,
        ));
    }
    y_offset += 2;

    // Content field
    let content_active = state.field == SearchField::Content && state.results.is_empty();
    let content_label_style = Style::default()
        .fg(if content_active {
            Color::Yellow
        } else {
            Color::Cyan
        })
        .bg(Color::Indexed(236));
    let content_input_style = Style::default().fg(Color::White).bg(if content_active {
        Color::Indexed(238)
    } else {
        Color::Indexed(237)
    });

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Containing text (optional):",
            content_label_style,
        ))),
        Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
    );
    y_offset += 1;

    let content_display = format!(
        " {:<width$}",
        state.content_query,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            content_display,
            content_input_style,
        ))),
        Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
    );
    if content_active {
        frame.set_cursor_position((
            inner.x + 1 + state.content_cursor as u16,
            inner.y + y_offset,
        ));
    }
    y_offset += 2;

    // Results area
    let results_height = inner.height.saturating_sub(y_offset + 1);

    if state.searching {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Searching...",
                Style::default().fg(Color::Yellow).bg(Color::Indexed(236)),
            )),
            Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
        );
    } else if !state.results.is_empty() {
        // Show result count
        let count_line = Line::from(Span::styled(
            format!(" Found {} file(s):", state.results.len()),
            Style::default().fg(Color::Green).bg(Color::Indexed(236)),
        ));
        frame.render_widget(
            Paragraph::new(count_line),
            Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
        );
        y_offset += 1;

        let visible = (results_height.saturating_sub(1)) as usize;
        // Adjust scroll
        let scroll = if state.result_cursor >= state.result_scroll + visible {
            state.result_cursor - visible + 1
        } else if state.result_cursor < state.result_scroll {
            state.result_cursor
        } else {
            state.result_scroll
        };

        for (i, result) in state.results.iter().skip(scroll).take(visible).enumerate() {
            let is_selected = scroll + i == state.result_cursor;
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Indexed(24))
            } else {
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
            };

            let prefix = if result.is_dir { "[DIR] " } else { "      " };
            let display = format!(" {}{}", prefix, result.path.display());
            let truncated: String = display.chars().take(inner.width as usize).collect();

            frame.render_widget(
                Paragraph::new(Span::styled(truncated, style)),
                Rect::new(inner.x, inner.y + y_offset + i as u16, inner.width, 1),
            );
        }
    }

    // Hint bar
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint = if state.results.is_empty() {
        " Enter=Search  Tab=Switch field  Esc=Close"
    } else {
        " Enter=Go to  Up/Down=Navigate  Tab=New search  Esc=Close"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
