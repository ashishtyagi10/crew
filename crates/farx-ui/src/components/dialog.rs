use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;

/// The type of dialog currently shown
#[derive(Debug, Clone)]
pub enum DialogKind {
    /// Input dialog for MkDir, Rename, etc.
    Input {
        title: String,
        prompt: String,
        input: String,
        cursor_pos: usize,
    },
    /// Confirmation dialog for Copy, Move, Delete
    Confirm {
        title: String,
        message: String,
        details: Vec<String>,
    },
    /// Message/alert dialog
    Message { title: String, message: String },
    /// Error dialog
    Error { title: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogResult {
    /// User confirmed (Enter) - for Input dialogs, contains the input string
    Confirm(Option<String>),
    /// User cancelled (Escape)
    Cancel,
    /// Dialog is still open
    Pending,
}

pub struct DialogState {
    pub kind: DialogKind,
    pub result: DialogResult,
}

impl DialogState {
    pub fn new_input(
        title: impl Into<String>,
        prompt: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        let input = default.into();
        let cursor_pos = input.len();
        Self {
            kind: DialogKind::Input {
                title: title.into(),
                prompt: prompt.into(),
                input,
                cursor_pos,
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_confirm(
        title: impl Into<String>,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self {
            kind: DialogKind::Confirm {
                title: title.into(),
                message: message.into(),
                details,
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_message(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: DialogKind::Message {
                title: title.into(),
                message: message.into(),
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: DialogKind::Error {
                title: title.into(),
                message: message.into(),
            },
            result: DialogResult::Pending,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match &mut self.kind {
            DialogKind::Input {
                input, cursor_pos, ..
            } => match key.code {
                KeyCode::Enter => {
                    self.result = DialogResult::Confirm(Some(input.clone()));
                }
                KeyCode::Esc => {
                    self.result = DialogResult::Cancel;
                }
                KeyCode::Char(ch) => {
                    input.insert(*cursor_pos, ch);
                    *cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                        input.remove(*cursor_pos);
                    }
                }
                KeyCode::Delete => {
                    if *cursor_pos < input.len() {
                        input.remove(*cursor_pos);
                    }
                }
                KeyCode::Left => {
                    *cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Right => {
                    *cursor_pos = (*cursor_pos + 1).min(input.len());
                }
                KeyCode::Home => {
                    *cursor_pos = 0;
                }
                KeyCode::End => {
                    *cursor_pos = input.len();
                }
                _ => {}
            },
            DialogKind::Confirm { .. } => match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.result = DialogResult::Confirm(None);
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.result = DialogResult::Cancel;
                }
                _ => {}
            },
            DialogKind::Message { .. } | DialogKind::Error { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.result = DialogResult::Cancel;
                }
                _ => {}
            },
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.result != DialogResult::Pending
    }
}

pub fn render_dialog(frame: &mut ratatui::Frame, state: &DialogState, _theme: &Theme) {
    let area = frame.area();

    // Dialog size: centered, dynamically sized
    let dialog_width = match &state.kind {
        DialogKind::Message { message, .. } | DialogKind::Error { message, .. } => {
            // Size to content, min 60, max screen-4
            let max_line = message.lines().map(|l| l.len()).max().unwrap_or(20);
            (max_line as u16 + 4).clamp(60, area.width.saturating_sub(4))
        }
        _ => 60u16.min(area.width.saturating_sub(4)),
    };
    let dialog_height = match &state.kind {
        DialogKind::Input { .. } => 7,
        DialogKind::Confirm { details, .. } => {
            (7 + details.len() as u16).min(area.height.saturating_sub(4))
        }
        DialogKind::Message { message, .. } | DialogKind::Error { message, .. } => {
            // Count lines in message, +4 for borders + hint + padding
            let line_count = message.lines().count().max(1) as u16;
            (line_count + 4).clamp(7, area.height.saturating_sub(4))
        }
    };

    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear the area behind the dialog
    frame.render_widget(Clear, dialog_area);

    match &state.kind {
        DialogKind::Input {
            title,
            prompt,
            input,
            cursor_pos,
        } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
                .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

            let inner = block.inner(dialog_area);
            frame.render_widget(block, dialog_area);

            // Prompt text
            let prompt_line = Line::from(vec![Span::styled(
                prompt.as_str(),
                Style::default().fg(Color::Cyan),
            )]);
            frame.render_widget(
                Paragraph::new(prompt_line),
                Rect {
                    y: inner.y,
                    height: 1,
                    ..inner
                },
            );

            // Input field with a visible background
            let input_area = Rect {
                x: inner.x,
                y: inner.y + 2,
                width: inner.width,
                height: 1,
            };
            let input_line = Line::from(vec![Span::styled(
                format!("{:<width$}", input, width = inner.width as usize),
                Style::default().fg(Color::White).bg(Color::Indexed(238)),
            )]);
            frame.render_widget(Paragraph::new(input_line), input_area);

            // Show cursor position
            frame.set_cursor_position((input_area.x + *cursor_pos as u16, input_area.y));

            // Hint at bottom
            let hint = Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw("=OK  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw("=Cancel"),
            ]);
            let hint_area = Rect {
                y: inner.y + inner.height.saturating_sub(1),
                height: 1,
                ..inner
            };
            frame.render_widget(Paragraph::new(hint), hint_area);
        }

        DialogKind::Confirm {
            title,
            message,
            details,
        } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
                .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

            let inner = block.inner(dialog_area);
            frame.render_widget(block, dialog_area);

            // Message
            let msg_line = Line::from(Span::styled(
                message.as_str(),
                Style::default().fg(Color::Cyan),
            ));
            frame.render_widget(
                Paragraph::new(msg_line),
                Rect {
                    y: inner.y,
                    height: 1,
                    ..inner
                },
            );

            // Detail lines (file names, etc.)
            for (i, detail) in details.iter().enumerate() {
                if i + 2 >= inner.height as usize {
                    break;
                }
                let detail_line = Line::from(Span::styled(
                    detail.as_str(),
                    Style::default().fg(Color::White),
                ));
                let detail_area = Rect {
                    y: inner.y + 1 + i as u16,
                    height: 1,
                    ..inner
                };
                frame.render_widget(Paragraph::new(detail_line), detail_area);
            }

            // Hint
            let hint = Line::from(vec![
                Span::styled("Enter/Y", Style::default().fg(Color::Yellow)),
                Span::raw("=Yes  "),
                Span::styled("Esc/N", Style::default().fg(Color::Yellow)),
                Span::raw("=No"),
            ]);
            let hint_area = Rect {
                y: inner.y + inner.height.saturating_sub(1),
                height: 1,
                ..inner
            };
            frame.render_widget(Paragraph::new(hint), hint_area);
        }

        DialogKind::Message { title, message } | DialogKind::Error { title, message } => {
            let is_error = matches!(state.kind, DialogKind::Error { .. });
            let border_color = if is_error { Color::Red } else { Color::Yellow };

            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(border_color).bg(Color::Indexed(236)))
                .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

            let inner = block.inner(dialog_area);
            frame.render_widget(block, dialog_area);

            let fg = if is_error { Color::Red } else { Color::Cyan };

            // Build multi-line content from the message
            let content_lines: Vec<Line> = message
                .lines()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(fg))))
                .collect();

            let content_area = Rect {
                y: inner.y,
                height: inner.height.saturating_sub(1),
                ..inner
            };
            frame.render_widget(Paragraph::new(content_lines), content_area);

            let hint = Line::from(Span::styled(
                "Press Enter or Esc to close",
                Style::default().fg(Color::DarkGray),
            ));
            let hint_area = Rect {
                y: inner.y + inner.height.saturating_sub(1),
                height: 1,
                ..inner
            };
            frame.render_widget(Paragraph::new(hint), hint_area);
        }
    }
}

/// Helper to create a centered rectangle within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
