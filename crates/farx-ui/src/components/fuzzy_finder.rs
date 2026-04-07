use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::path::{Path, PathBuf};

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum FuzzyAction {
    None,
    Close,
    /// Navigate to this path's parent directory
    GoTo(PathBuf),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct FuzzyResult {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) rel_path: String,
    pub(crate) score: i32,
}

pub struct FuzzyFinderState {
    pub active: bool,
    pub query: String,
    pub cursor_pos: usize,
    #[allow(private_interfaces)]
    pub results: Vec<FuzzyResult>,
    pub result_cursor: usize,
    pub result_scroll: usize,
    #[allow(dead_code)]
    pub(crate) root: PathBuf,
    all_files: Vec<(PathBuf, String, String)>, // (abs_path, name, rel_path)
}

impl FuzzyFinderState {
    pub fn new(root: PathBuf) -> Self {
        let mut state = Self {
            active: true,
            query: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            result_cursor: 0,
            result_scroll: 0,
            root: root.clone(),
            all_files: Vec::new(),
        };
        state.scan_files(&root, &root, 0);
        state.results = state
            .all_files
            .iter()
            .take(100)
            .map(|(p, n, r)| FuzzyResult {
                path: p.clone(),
                name: n.clone(),
                rel_path: r.clone(),
                score: 0,
            })
            .collect();
        state
    }

    fn scan_files(&mut self, dir: &Path, root: &Path, depth: usize) {
        if depth > 8 || self.all_files.len() > 10_000 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            self.all_files.push((path.clone(), name, rel));
            if path.is_dir() {
                self.scan_files(&path, root, depth + 1);
            }
        }
    }

    fn update_results(&mut self) {
        if self.query.is_empty() {
            self.results = self
                .all_files
                .iter()
                .take(100)
                .map(|(p, n, r)| FuzzyResult {
                    path: p.clone(),
                    name: n.clone(),
                    rel_path: r.clone(),
                    score: 0,
                })
                .collect();
        } else {
            let query_lower = self.query.to_lowercase();
            let mut scored: Vec<FuzzyResult> = self
                .all_files
                .iter()
                .filter_map(|(p, n, r)| {
                    let score = fuzzy_score(&r.to_lowercase(), &query_lower);
                    if score > 0 {
                        Some(FuzzyResult {
                            path: p.clone(),
                            name: n.clone(),
                            rel_path: r.clone(),
                            score,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by(|a, b| b.score.cmp(&a.score));
            scored.truncate(100);
            self.results = scored;
        }
        self.result_cursor = 0;
        self.result_scroll = 0;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> FuzzyAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                FuzzyAction::Close
            }
            KeyCode::Enter => {
                if let Some(result) = self.results.get(self.result_cursor) {
                    let path = result.path.clone();
                    self.active = false;
                    FuzzyAction::GoTo(path)
                } else {
                    FuzzyAction::None
                }
            }
            KeyCode::Up => {
                if self.result_cursor > 0 {
                    self.result_cursor -= 1;
                    if self.result_cursor < self.result_scroll {
                        self.result_scroll = self.result_cursor;
                    }
                }
                FuzzyAction::None
            }
            KeyCode::Down => {
                if self.result_cursor + 1 < self.results.len() {
                    self.result_cursor += 1;
                }
                FuzzyAction::None
            }
            KeyCode::Char(ch) => {
                self.query.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
                self.update_results();
                FuzzyAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                    self.update_results();
                }
                FuzzyAction::None
            }
            _ => FuzzyAction::None,
        }
    }
}

/// Simple fuzzy scoring: characters from query must appear in order in the text.
/// Higher score for consecutive matches and matches at word boundaries.
fn fuzzy_score(text: &str, query: &str) -> i32 {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return 1;
    }

    let mut score = 0i32;
    let mut qi = 0;
    let mut prev_match = false;

    for (ti, &tc) in text_chars.iter().enumerate() {
        if qi < query_chars.len() && tc == query_chars[qi] {
            score += 1;
            // Bonus for consecutive matches
            if prev_match {
                score += 2;
            }
            // Bonus for match at start or after separator
            if ti == 0
                || matches!(
                    text_chars.get(ti.wrapping_sub(1)),
                    Some('/' | '\\' | '_' | '-' | '.')
                )
            {
                score += 3;
            }
            qi += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if qi == query_chars.len() {
        score
    } else {
        0 // Not all query chars matched
    }
}

pub fn render_fuzzy_finder(frame: &mut Frame, state: &FuzzyFinderState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " Find File (Ctrl+P) — {} files ",
            state.all_files.len()
        ))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Query input
    let query_display = format!(
        " {:<width$}",
        state.query,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            query_display,
            Style::default().fg(Color::White).bg(Color::Indexed(238)),
        ))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.set_cursor_position((inner.x + 1 + state.cursor_pos as u16, inner.y));

    // Results count
    let count_line = Line::from(Span::styled(
        format!(" {} matches", state.results.len()),
        Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
    ));
    frame.render_widget(
        Paragraph::new(count_line),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Results list
    let list_start = inner.y + 2;
    let visible = (inner.height.saturating_sub(3)) as usize;

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
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(24))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let icon = if result.path.is_dir() { "[D] " } else { "    " };
        let display = format!(" {}{}", icon, result.rel_path);
        let truncated: String = display.chars().take(inner.width as usize).collect();
        frame.render_widget(
            Paragraph::new(Span::styled(truncated, style)),
            Rect::new(inner.x, list_start + i as u16, inner.width, 1),
        );
    }

    // Hint
    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Enter=Go  Up/Down=Navigate  Esc=Close",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
