use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::state::{ActiveField, BatchRenameState};
use crate::theme::Theme;

pub fn render_batch_rename(frame: &mut Frame, state: &BatchRenameState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Batch Rename ({} files) ", state.files.len()))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut y_off = 0u16;
    y_off = render_field(
        frame,
        inner,
        y_off,
        " Find (regex):",
        &state.find_pattern,
        state.find_cursor,
        state.field == ActiveField::Find,
    );
    y_off = render_field(
        frame,
        inner,
        y_off,
        " Replace with:",
        &state.replace_pattern,
        state.replace_cursor,
        state.field == ActiveField::Replace,
    );

    render_preview(frame, inner, y_off, state);
    render_hint(frame, inner);
}

fn render_field(
    frame: &mut Frame,
    inner: Rect,
    mut y_off: u16,
    label: &str,
    value: &str,
    cursor: usize,
    active: bool,
) -> u16 {
    let label_style = Style::default()
        .fg(if active { Color::Yellow } else { Color::Cyan })
        .bg(Color::Indexed(236));
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(label.to_string(), label_style))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    y_off += 1;

    let input_style = Style::default().fg(Color::White).bg(if active {
        Color::Indexed(238)
    } else {
        Color::Indexed(237)
    });
    let display = format!(
        " {:<width$}",
        value,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(display, input_style))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    if active {
        frame.set_cursor_position((inner.x + 1 + cursor as u16, inner.y + y_off));
    }
    y_off + 2
}

fn render_preview(frame: &mut Frame, inner: Rect, mut y_off: u16, state: &BatchRenameState) {
    let preview_height = inner.height.saturating_sub(y_off + 1) as usize;
    let half_w = (inner.width as usize).saturating_sub(4) / 2;

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Preview:",
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
        ))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    y_off += 1;

    for (i, ((_, old_name), new_name)) in state
        .files
        .iter()
        .zip(state.previews.iter())
        .skip(state.scroll)
        .take(preview_height)
        .enumerate()
    {
        let changed = old_name != new_name;
        let old_trunc: String = old_name.chars().take(half_w).collect();
        let new_trunc: String = new_name.chars().take(half_w).collect();
        let arrow = if changed { " → " } else { "   " };

        let old_style = Style::default().fg(Color::White).bg(Color::Indexed(236));
        let new_style = if changed {
            Style::default()
                .fg(Color::Green)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
        };

        let line = Line::from(vec![
            Span::styled(format!(" {:<w$}", old_trunc, w = half_w), old_style),
            Span::styled(
                arrow,
                Style::default().fg(Color::Yellow).bg(Color::Indexed(236)),
            ),
            Span::styled(format!("{:<w$}", new_trunc, w = half_w), new_style),
        ]);
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(inner.x, inner.y + y_off + i as u16, inner.width, 1),
        );
    }
}

fn render_hint(frame: &mut Frame, inner: Rect) {
    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Tab=Switch field  Enter=Apply  Esc=Cancel",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
