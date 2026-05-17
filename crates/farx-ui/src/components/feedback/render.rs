//! Top-level feedback rendering dispatch and inline message/confirm rendering.

use std::time::{Duration, Instant};

use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::output_panel::render_output_panel;
use super::state::FeedbackState;
use super::types::{FeedbackKind, FeedbackMessage, InlineConfirm};

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
