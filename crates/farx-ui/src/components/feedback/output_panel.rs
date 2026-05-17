//! Scrollable output panel rendering.

use ratatui::prelude::*;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::state::FeedbackState;

pub(super) fn render_output_panel(frame: &mut Frame, area: Rect, state: &FeedbackState) {
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
