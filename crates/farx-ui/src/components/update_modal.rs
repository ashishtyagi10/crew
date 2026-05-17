use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use farx_core::update::UpdateStatus;

use crate::theme::Theme;

/// Lifecycle of the in-TUI update flow.
pub enum UpdateState {
    /// Background check in flight; poll `rx` until it yields a result.
    Checking { rx: mpsc::Receiver<UpdateStatus> },
    /// Newer version found; awaiting Y/N from the user.
    Confirm { current: String, latest: String },
    /// User confirmed; main loop is leaving the alt screen to run the installer.
    Installing { latest: String },
    /// Install succeeded.
    Done { version: String },
    /// Check or install failed.
    Failed { message: String },
}

/// What the user did with the modal this tick.
pub enum UpdateAction {
    None,
    Confirmed,
    Cancelled,
    Dismissed,
}

impl UpdateState {
    /// True when this state renders a modal that should swallow key events.
    pub fn is_modal(&self) -> bool {
        matches!(
            self,
            UpdateState::Confirm { .. }
                | UpdateState::Installing { .. }
                | UpdateState::Done { .. }
                | UpdateState::Failed { .. }
        )
    }

    pub fn handle_key_event(&self, key: KeyEvent) -> UpdateAction {
        match self {
            UpdateState::Confirm { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => UpdateAction::Confirmed,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => UpdateAction::Cancelled,
                _ => UpdateAction::None,
            },
            UpdateState::Done { .. } | UpdateState::Failed { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => UpdateAction::Dismissed,
                _ => UpdateAction::None,
            },
            UpdateState::Installing { .. } | UpdateState::Checking { .. } => UpdateAction::None,
        }
    }
}

/// Render the modal for the current state. `Checking` has no modal — feedback line covers it.
pub fn render_update_modal(frame: &mut Frame, state: &UpdateState, _theme: &Theme) {
    let (title, body, prompt, accent) = match state {
        UpdateState::Checking { .. } => return,
        UpdateState::Confirm { current, latest } => (
            " Update available ",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  A new version of farx is available.",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  current: ", Style::default().fg(Color::Indexed(244))),
                    Span::styled(format!("v{}", current), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("  latest:  ", Style::default().fg(Color::Indexed(244))),
                    Span::styled(
                        format!("v{}", latest),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ],
            "[Y] Install    [N] Cancel",
            Color::Cyan,
        ),
        UpdateState::Installing { latest } => (
            " Installing… ",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Installing v{}…", latest),
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Download progress is shown in the terminal below.",
                    Style::default().fg(Color::Indexed(244)),
                )),
            ],
            "",
            Color::Yellow,
        ),
        UpdateState::Done { version } => (
            " Updated ",
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Installed ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("v{}", version),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(".", Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Restart farx to use the new version.",
                    Style::default().fg(Color::Indexed(250)),
                )),
            ],
            "[Enter] Dismiss",
            Color::Green,
        ),
        UpdateState::Failed { message } => (
            " Update failed ",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Could not complete the update:",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", message),
                    Style::default().fg(Color::Red),
                )),
            ],
            "[Enter] Dismiss",
            Color::Red,
        ),
    };

    let area = frame.area();
    let width = 56u16.min(area.width.saturating_sub(4));
    let height = (body.len() as u16 + if prompt.is_empty() { 4 } else { 5 }).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(Color::Indexed(236)));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Body
    let body_height = inner
        .height
        .saturating_sub(if prompt.is_empty() { 0 } else { 2 });
    let body_area = Rect::new(inner.x, inner.y, inner.width, body_height);
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        body_area,
    );

    // Prompt strip at the bottom
    if !prompt.is_empty() {
        let prompt_area = Rect::new(inner.x, inner.y + body_height, inner.width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                prompt,
                Style::default()
                    .fg(Color::Black)
                    .bg(accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center),
            prompt_area,
        );
    }
}
