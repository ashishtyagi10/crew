use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Result of the chmod dialog interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ChmodAction {
    None,
    /// Apply the new mode.
    Apply(u32),
    Cancel,
}

/// State for the chmod / file permissions dialog.
pub struct ChmodDialogState {
    /// The file path being modified.
    pub file_path: std::path::PathBuf,
    /// Permission bits as a 9-element array: [owner_r, owner_w, owner_x, group_r, group_w, group_x, other_r, other_w, other_x].
    pub bits: [bool; 9],
    /// Currently focused bit index (0..8).
    pub cursor: usize,
}

impl ChmodDialogState {
    /// Create a new chmod dialog from a Unix mode value.
    pub fn new(file_path: std::path::PathBuf, mode: u32) -> Self {
        let bits = [
            mode & 0o400 != 0, // owner read
            mode & 0o200 != 0, // owner write
            mode & 0o100 != 0, // owner execute
            mode & 0o040 != 0, // group read
            mode & 0o020 != 0, // group write
            mode & 0o010 != 0, // group execute
            mode & 0o004 != 0, // other read
            mode & 0o002 != 0, // other write
            mode & 0o001 != 0, // other execute
        ];
        Self {
            file_path,
            bits,
            cursor: 0,
        }
    }

    /// Convert the bits array back to a Unix mode value (lower 9 bits).
    pub fn to_mode(&self) -> u32 {
        let mut mode = 0u32;
        let masks = [
            0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
        ];
        for (i, &set) in self.bits.iter().enumerate() {
            if set {
                mode |= masks[i];
            }
        }
        mode
    }

    /// Handle a key event, returning the action to take.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ChmodAction {
        match key.code {
            KeyCode::Esc => ChmodAction::Cancel,
            KeyCode::Enter => ChmodAction::Apply(self.to_mode()),
            KeyCode::Char(' ') => {
                self.bits[self.cursor] = !self.bits[self.cursor];
                ChmodAction::None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                ChmodAction::None
            }
            KeyCode::Right => {
                if self.cursor < 8 {
                    self.cursor += 1;
                }
                ChmodAction::None
            }
            KeyCode::Up => {
                // Move up one row (3 columns per row)
                if self.cursor >= 3 {
                    self.cursor -= 3;
                }
                ChmodAction::None
            }
            KeyCode::Down => {
                // Move down one row
                if self.cursor + 3 <= 8 {
                    self.cursor += 3;
                }
                ChmodAction::None
            }
            KeyCode::Tab => {
                // Cycle through all 9 positions
                self.cursor = (self.cursor + 1) % 9;
                ChmodAction::None
            }
            _ => ChmodAction::None,
        }
    }
}

/// Render the chmod dialog.
pub fn render_chmod_dialog(frame: &mut Frame, state: &ChmodDialogState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 44u16.min(area.width.saturating_sub(4));
    let dialog_height = 12u16.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let filename = state
        .file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let title = format!(" Permissions: {} ", filename);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let labels = ["Read", "Write", "Execute"];
    let groups = ["Owner", "Group", "Other"];

    // Header row
    let mut header_spans = vec![Span::styled(
        format!("{:<10}", ""),
        Style::default().fg(Color::White).bg(Color::Indexed(236)),
    )];
    for label in &labels {
        header_spans.push(Span::styled(
            format!(" {:<9}", label),
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(header_spans)),
        Rect {
            y: inner.y,
            height: 1,
            ..inner
        },
    );

    // Permission rows
    for (row, group) in groups.iter().enumerate() {
        let mut spans = vec![Span::styled(
            format!(" {:<9}", group),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        )];

        for col in 0..3 {
            let bit_idx = row * 3 + col;
            let is_focused = state.cursor == bit_idx;
            let is_set = state.bits[bit_idx];

            let checkbox = if is_set { "[x]" } else { "[ ]" };

            let style = if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_set {
                Style::default().fg(Color::Green).bg(Color::Indexed(236))
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
            };

            spans.push(Span::styled(format!("  {:<7}", checkbox), style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                y: inner.y + 1 + row as u16,
                height: 1,
                ..inner
            },
        );
    }

    // Octal display
    let mode = state.to_mode();
    let octal_line = Line::from(vec![
        Span::styled(
            " Octal: ",
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        ),
        Span::styled(
            format!("{:04o}", mode),
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", format_rwx(mode)),
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(octal_line),
        Rect {
            y: inner.y + 5,
            height: 1,
            ..inner
        },
    );

    // Hint
    let hint = Line::from(vec![
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw("=Toggle  "),
        Span::styled("Arrows", Style::default().fg(Color::Yellow)),
        Span::raw("=Move  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw("=Apply  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw("=Cancel"),
    ]);
    frame.render_widget(
        Paragraph::new(hint),
        Rect {
            y: inner.y + inner.height.saturating_sub(1),
            height: 1,
            ..inner
        },
    );
}

/// Format a mode as rwxrwxrwx string.
fn format_rwx(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let chars = ['r', 'w', 'x'];
    let masks = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    for (i, &mask) in masks.iter().enumerate() {
        if mode & mask != 0 {
            s.push(chars[i % 3]);
        } else {
            s.push('-');
        }
    }
    s
}
