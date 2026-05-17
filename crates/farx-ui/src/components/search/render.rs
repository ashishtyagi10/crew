use super::action::SearchField;
use super::render_results::render_results_and_hint;
use super::state::SearchState;
use crate::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

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

    y_offset = render_pattern_field(frame, state, inner, y_offset);
    y_offset = render_content_field(frame, state, inner, y_offset);

    render_results_and_hint(frame, state, inner, y_offset);
}

fn render_pattern_field(
    frame: &mut Frame,
    state: &SearchState,
    inner: Rect,
    mut y_offset: u16,
) -> u16 {
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
    y_offset + 2
}

fn render_content_field(
    frame: &mut Frame,
    state: &SearchState,
    inner: Rect,
    mut y_offset: u16,
) -> u16 {
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
    y_offset + 2
}
