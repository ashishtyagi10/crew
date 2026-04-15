use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum DiffAction {
    None,
    Close,
}

/// A single line in the diff output.
#[derive(Debug, Clone)]
enum DiffLine {
    /// Line exists in both files and is identical.
    Same(String),
    /// Line was added (only in right file).
    Added(String),
    /// Line was removed (only in left file).
    Removed(String),
    /// Line was changed (different in left and right).
    Changed(String, String),
}

pub struct DiffViewState {
    pub left_path: std::path::PathBuf,
    pub right_path: std::path::PathBuf,
    diff_lines: Vec<DiffLine>,
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

/// Simple LCS-based diff algorithm.
fn compute_diff(left: &[&str], right: &[&str]) -> Vec<DiffLine> {
    let m = left.len();
    let n = right.len();

    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if left[i - 1] == right[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;

    let mut stack = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left[i - 1] == right[j - 1] {
            stack.push(DiffLine::Same(left[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            stack.push(DiffLine::Added(right[j - 1].to_string()));
            j -= 1;
        } else if i > 0 {
            stack.push(DiffLine::Removed(left[i - 1].to_string()));
            i -= 1;
        }
    }

    // Reverse since we built it backwards
    stack.reverse();

    // Merge adjacent Removed+Added into Changed
    let mut idx = 0;
    while idx < stack.len() {
        if idx + 1 < stack.len() {
            if let (DiffLine::Removed(ref l), DiffLine::Added(ref r)) =
                (&stack[idx], &stack[idx + 1])
            {
                result.push(DiffLine::Changed(l.clone(), r.clone()));
                idx += 2;
                continue;
            }
        }
        result.push(stack[idx].clone());
        idx += 1;
    }

    result
}

pub fn render_diff_view(frame: &mut Frame, state: &DiffViewState, _theme: &Theme) {
    let area = frame.area();

    let left_name = state
        .left_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let right_name = state
        .right_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let title = format!(" Diff: {} ↔ {} ", left_name, right_name);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Indexed(233)))
        .style(Style::default().bg(Color::Indexed(233)).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    // Reserve 1 line for hint bar
    let content_height = (inner.height - 1) as usize;
    let half_width = (inner.width / 2) as usize;

    // Header: left filename | right filename
    let header = Line::from(vec![
        Span::styled(
            format!(" {:<width$}", left_name, width = half_width - 2),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(235))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "│",
            Style::default()
                .fg(Color::Rgb(60, 60, 65))
                .bg(Color::Indexed(235)),
        ),
        Span::styled(
            format!(" {}", right_name),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(235))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header),
        Rect {
            y: inner.y,
            height: 1,
            ..inner
        },
    );

    // Diff lines
    let visible = content_height.saturating_sub(1);
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(visible);

    for idx in state.scroll_offset..(state.scroll_offset + visible).min(state.diff_lines.len()) {
        let line = &state.diff_lines[idx];
        let row = match line {
            DiffLine::Same(text) => {
                let left_text = truncate_pad(text, half_width - 1);
                let right_text = truncate_pad(text, half_width - 1);
                Line::from(vec![
                    Span::styled(
                        format!(" {}", left_text),
                        Style::default().fg(Color::White).bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        "│",
                        Style::default()
                            .fg(Color::Rgb(60, 60, 65))
                            .bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        format!(" {}", right_text),
                        Style::default().fg(Color::White).bg(Color::Indexed(233)),
                    ),
                ])
            }
            DiffLine::Removed(text) => {
                let left_text = truncate_pad(text, half_width - 1);
                let right_text = " ".repeat(half_width - 1);
                Line::from(vec![
                    Span::styled(
                        format!("-{}", left_text),
                        Style::default().fg(Color::Red).bg(Color::Indexed(52)),
                    ),
                    Span::styled(
                        "│",
                        Style::default()
                            .fg(Color::Rgb(60, 60, 65))
                            .bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        format!(" {}", right_text),
                        Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
                    ),
                ])
            }
            DiffLine::Added(text) => {
                let left_text = " ".repeat(half_width - 1);
                let right_text = truncate_pad(text, half_width - 1);
                Line::from(vec![
                    Span::styled(
                        format!(" {}", left_text),
                        Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        "│",
                        Style::default()
                            .fg(Color::Rgb(60, 60, 65))
                            .bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        format!("+{}", right_text),
                        Style::default().fg(Color::Green).bg(Color::Indexed(22)),
                    ),
                ])
            }
            DiffLine::Changed(left, right) => {
                let left_text = truncate_pad(left, half_width - 1);
                let right_text = truncate_pad(right, half_width - 1);
                Line::from(vec![
                    Span::styled(
                        format!("~{}", left_text),
                        Style::default().fg(Color::Red).bg(Color::Indexed(52)),
                    ),
                    Span::styled(
                        "│",
                        Style::default()
                            .fg(Color::Rgb(60, 60, 65))
                            .bg(Color::Indexed(233)),
                    ),
                    Span::styled(
                        format!("~{}", right_text),
                        Style::default().fg(Color::Green).bg(Color::Indexed(22)),
                    ),
                ])
            }
        };
        lines.push(row);
    }

    // Fill remaining lines
    while lines.len() < visible {
        lines.push(Line::from(Span::styled(
            " ".repeat(inner.width as usize),
            Style::default().bg(Color::Indexed(233)),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines),
        Rect {
            y: inner.y + 1,
            height: visible as u16,
            ..inner
        },
    );

    // Hint bar
    let total = state.diff_lines.len();
    let pos = if total > 0 {
        state.scroll_offset + 1
    } else {
        0
    };
    let hint = format!(
        " Line {}/{} | Up/Down/PgUp/PgDn=Scroll | Esc=Close",
        pos, total
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
        )),
        Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        },
    );
}

fn truncate_pad(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= width {
        let mut s: String = chars[..width.saturating_sub(1)].iter().collect();
        s.push('~');
        s
    } else {
        format!("{:<width$}", text, width = width)
    }
}
