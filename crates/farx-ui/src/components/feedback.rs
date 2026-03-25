//! Inline feedback system — replaces all modal dialogs.
//!
//! Messages appear in the command bar area and auto-dismiss.
//! Confirmations are inline Y/N prompts that resolve without blocking.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::time::{Duration, Instant};

/// A single feedback message
#[derive(Debug, Clone)]
pub struct FeedbackMessage {
    pub kind: FeedbackKind,
    pub text: String,
    pub created: Instant,
    pub ttl: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackKind {
    /// Green success message
    Success,
    /// Red error message
    Error,
    /// Yellow warning message
    Warning,
    /// Cyan info message
    Info,
    /// Scrollable output (e.g. command output)
    Output,
}

/// Inline confirmation state
#[derive(Debug, Clone)]
pub struct InlineConfirm {
    pub prompt: String,
    pub detail: String,
    pub action_id: ConfirmAction,
    pub created: Instant,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Copy {
        sources: Vec<std::path::PathBuf>,
        dest: std::path::PathBuf,
    },
    Move {
        sources: Vec<std::path::PathBuf>,
        dest: std::path::PathBuf,
    },
    Delete {
        targets: Vec<std::path::PathBuf>,
    },
}

/// Result of handling a key in the feedback system
#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackResult {
    /// Key was not consumed
    NotHandled,
    /// Key was consumed, no action needed
    Consumed,
    /// Confirmation was accepted
    Confirmed(usize), // index into confirm queue
    /// Confirmation was rejected
    Rejected,
}

/// The feedback system state
pub struct FeedbackState {
    /// Queue of messages (newest last)
    pub messages: Vec<FeedbackMessage>,
    /// Current inline confirmation, if any
    pub confirm: Option<InlineConfirm>,
    /// Scrollable output lines (for command output)
    pub output_lines: Vec<String>,
    pub output_scroll: usize,
    pub output_title: String,
    /// Whether output panel is visible
    pub output_visible: bool,
}

impl FeedbackState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            confirm: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_title: String::new(),
            output_visible: false,
        }
    }

    /// Push a success message (auto-dismiss 3s)
    pub fn success(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Success, text.into(), Duration::from_secs(3));
    }

    /// Push an error message (auto-dismiss 5s)
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Error, text.into(), Duration::from_secs(5));
    }

    /// Push a warning message (auto-dismiss 4s)
    pub fn warning(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Warning, text.into(), Duration::from_secs(4));
    }

    /// Push an info message (auto-dismiss 3s)
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Info, text.into(), Duration::from_secs(3));
    }

    fn push(&mut self, kind: FeedbackKind, text: String, ttl: Duration) {
        // Only keep last 5 messages
        if self.messages.len() >= 5 {
            self.messages.remove(0);
        }
        self.messages.push(FeedbackMessage {
            kind,
            text,
            created: Instant::now(),
            ttl,
        });
    }

    /// Show scrollable output (e.g. command results)
    pub fn show_output(&mut self, title: impl Into<String>, text: String) {
        self.output_title = title.into();
        self.output_lines = text.lines().map(String::from).collect();
        self.output_scroll = 0;
        self.output_visible = true;
    }

    /// Request an inline confirmation
    pub fn ask_confirm(
        &mut self,
        prompt: impl Into<String>,
        detail: impl Into<String>,
        action: ConfirmAction,
    ) {
        self.confirm = Some(InlineConfirm {
            prompt: prompt.into(),
            detail: detail.into(),
            action_id: action,
            created: Instant::now(),
        });
    }

    /// Tick: remove expired messages, auto-dismiss output after inactivity
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.messages
            .retain(|m| now.duration_since(m.created) < m.ttl);

        // Auto-dismiss output panel after 30 seconds
        if self.output_visible && !self.output_lines.is_empty() {
            // Keep it visible until user acts
        }
    }

    /// Handle a key event. Returns whether it was consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> FeedbackResult {
        // Confirmation takes priority
        if self.confirm.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    return FeedbackResult::Confirmed(0);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirm = None;
                    return FeedbackResult::Rejected;
                }
                _ => return FeedbackResult::Consumed,
            }
        }

        // Output panel scroll
        if self.output_visible {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    self.output_visible = false;
                    self.output_lines.clear();
                    return FeedbackResult::Consumed;
                }
                KeyCode::Up => {
                    self.output_scroll = self.output_scroll.saturating_sub(1);
                    return FeedbackResult::Consumed;
                }
                KeyCode::Down => {
                    if self.output_scroll + 1 < self.output_lines.len() {
                        self.output_scroll += 1;
                    }
                    return FeedbackResult::Consumed;
                }
                KeyCode::PageUp => {
                    self.output_scroll = self.output_scroll.saturating_sub(20);
                    return FeedbackResult::Consumed;
                }
                KeyCode::PageDown => {
                    self.output_scroll =
                        (self.output_scroll + 20).min(self.output_lines.len().saturating_sub(1));
                    return FeedbackResult::Consumed;
                }
                _ => {
                    // Any other key dismisses the output
                    self.output_visible = false;
                    self.output_lines.clear();
                    return FeedbackResult::Consumed;
                }
            }
        }

        // Any keypress clears stale messages
        if !self.messages.is_empty() {
            self.messages.clear();
        }

        FeedbackResult::NotHandled
    }

    /// Check if feedback has anything to show
    pub fn has_content(&self) -> bool {
        !self.messages.is_empty() || self.confirm.is_some() || self.output_visible
    }

    /// Take the confirmed action (consumes it)
    pub fn take_confirm(&mut self) -> Option<ConfirmAction> {
        self.confirm.take().map(|c| c.action_id)
    }
}

impl Default for FeedbackState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the feedback area. This replaces the command line when feedback is active.
/// Returns the height consumed (0 if no feedback, or height of output panel).
pub fn render_feedback(frame: &mut Frame, area: Rect, state: &FeedbackState) -> u16 {
    // Scrollable output panel (takes variable height above the command line)
    if state.output_visible && !state.output_lines.is_empty() {
        render_output_panel(frame, area, state);
        return area.height;
    }

    // Inline confirmation
    if let Some(ref confirm) = state.confirm {
        render_confirm(frame, area, confirm);
        return area.height;
    }

    // Inline messages
    if let Some(msg) = state.messages.last() {
        render_message(frame, area, msg);
        return 1;
    }

    0
}

fn render_message(frame: &mut Frame, area: Rect, msg: &FeedbackMessage) {
    let (icon, fg) = match msg.kind {
        FeedbackKind::Success => ("✓", Color::Rgb(120, 190, 90)),
        FeedbackKind::Error => ("✗", Color::Rgb(230, 80, 80)),
        FeedbackKind::Warning => ("⚠", Color::Rgb(230, 190, 110)),
        FeedbackKind::Info => ("●", Color::Rgb(90, 180, 160)),
        FeedbackKind::Output => ("▸", Color::Rgb(190, 186, 178)),
    };

    // Calculate remaining time for fade effect
    let elapsed = Instant::now().duration_since(msg.created);
    let remaining = msg.ttl.saturating_sub(elapsed);
    let fade = if remaining < Duration::from_millis(500) {
        // Dim in last 500ms
        Modifier::DIM
    } else {
        Modifier::empty()
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", icon),
            Style::default()
                .fg(fg)
                .bg(Color::Rgb(16, 16, 18))
                .add_modifier(fade),
        ),
        Span::styled(
            msg.text.clone(),
            Style::default()
                .fg(fg)
                .bg(Color::Rgb(16, 16, 18))
                .add_modifier(fade),
        ),
    ]);

    let msg_area = Rect { height: 1, ..area };
    frame.render_widget(Paragraph::new(line), msg_area);
}

fn render_confirm(frame: &mut Frame, area: Rect, confirm: &InlineConfirm) {
    let bg = Color::Rgb(30, 28, 20);

    let line = Line::from(vec![
        Span::styled(
            " ⚡ ",
            Style::default()
                .fg(Color::Rgb(230, 190, 110))
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            confirm.prompt.clone(),
            Style::default().fg(Color::Rgb(230, 190, 110)).bg(bg),
        ),
        Span::styled(
            format!(" {} ", confirm.detail),
            Style::default().fg(Color::Rgb(190, 186, 178)).bg(bg),
        ),
        Span::styled(
            " [Y]es ",
            Style::default()
                .fg(Color::Rgb(120, 190, 90))
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " [N]o ",
            Style::default()
                .fg(Color::Rgb(230, 80, 80))
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(bg)),
        Rect { height: 1, ..area },
    );
}

fn render_output_panel(frame: &mut Frame, area: Rect, state: &FeedbackState) {
    let bg = Color::Rgb(20, 20, 24);
    let border_color = Color::Rgb(60, 60, 65);

    // Use available height, max 60% of area
    let max_lines = (area.height as usize * 60) / 100;
    let content_lines = state.output_lines.len().min(max_lines).max(3);
    let panel_height = (content_lines as u16 + 2).min(area.height); // +2 for border

    let panel_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(panel_height),
        width: area.width,
        height: panel_height,
    };

    // Clear area
    frame.render_widget(ratatui::widgets::Clear, panel_area);

    // Border with title
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .title(Span::styled(
            format!(" {} ", state.output_title),
            Style::default().fg(Color::Rgb(220, 170, 60)).bg(bg),
        ))
        .title_bottom(Line::from(vec![
            Span::styled(" Esc", Style::default().fg(Color::Rgb(220, 170, 60)).bg(bg)),
            Span::styled(
                "=close  ",
                Style::default().fg(Color::Rgb(90, 90, 110)).bg(bg),
            ),
            Span::styled("↑↓", Style::default().fg(Color::Rgb(220, 170, 60)).bg(bg)),
            Span::styled(
                "=scroll ",
                Style::default().fg(Color::Rgb(90, 90, 110)).bg(bg),
            ),
        ]))
        .border_style(Style::default().fg(border_color).bg(bg))
        .style(Style::default().bg(bg));

    let inner = block.inner(panel_area);
    frame.render_widget(block, panel_area);

    // Render lines
    let visible: Vec<Line> = state
        .output_lines
        .iter()
        .skip(state.output_scroll)
        .take(inner.height as usize)
        .map(|l| {
            Line::from(Span::styled(
                format!(" {}", l),
                Style::default().fg(Color::Rgb(190, 186, 178)).bg(bg),
            ))
        })
        .collect();

    frame.render_widget(Paragraph::new(visible), inner);
}
