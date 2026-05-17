use super::render_response::{render_hint_bar, render_response_area};
use super::state::AiBarState;
use crate::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

#[allow(unused_variables)]
pub fn render_ai_bar(frame: &mut Frame, state: &AiBarState, theme: &Theme) {
    let area = frame.area();

    // AI bar takes the bottom half of the screen
    let bar_height = (area.height / 2).max(8);
    let bar_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(bar_height),
        area.width,
        bar_height,
    );

    frame.render_widget(Clear, bar_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI Assistant (Ctrl+Space) ")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Rgb(135, 215, 255))
                .bg(Color::Indexed(234)),
        )
        .style(
            Style::default()
                .bg(Color::Indexed(234))
                .fg(Color::Rgb(135, 215, 255)),
        );

    let inner = block.inner(bar_area);
    frame.render_widget(block, bar_area);

    // Input line at top
    let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let prompt_style = Style::default()
        .fg(Color::Rgb(255, 175, 0))
        .bg(Color::Indexed(234));
    let input_style = Style::default().fg(Color::White).bg(Color::Indexed(236));

    let input_display = format!(
        "{:<width$}",
        state.input,
        width = (inner.width as usize).saturating_sub(4)
    );
    let input_line = Line::from(vec![
        Span::styled(" > ", prompt_style),
        Span::styled(input_display, input_style),
    ]);
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Set cursor position in input
    frame.set_cursor_position((inner.x + 3 + state.cursor_pos as u16, inner.y));

    // Separator
    let sep_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let sep_line = Line::from(Span::styled(
        "\u{2500}".repeat(inner.width as usize),
        Style::default()
            .fg(Color::Indexed(240))
            .bg(Color::Indexed(234)),
    ));
    frame.render_widget(Paragraph::new(sep_line), sep_area);

    // Response area
    let response_area = Rect::new(
        inner.x,
        inner.y + 2,
        inner.width,
        inner.height.saturating_sub(3),
    );
    render_response_area(frame, state, response_area);

    // Bottom hint bar
    let hint_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1),
        inner.width,
        1,
    );
    render_hint_bar(frame, state, hint_area);
}
