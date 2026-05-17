use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub(super) fn render_input(
    frame: &mut ratatui::Frame,
    dialog_area: Rect,
    title: &str,
    prompt: &str,
    input: &str,
    cursor_pos: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Prompt text
    let prompt_line = Line::from(vec![Span::styled(prompt, Style::default().fg(Color::Cyan))]);
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
    frame.set_cursor_position((input_area.x + cursor_pos as u16, input_area.y));

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

pub(super) fn render_confirm(
    frame: &mut ratatui::Frame,
    dialog_area: Rect,
    title: &str,
    message: &str,
    details: &[String],
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Message
    let msg_line = Line::from(Span::styled(message, Style::default().fg(Color::Cyan)));
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

pub(super) fn render_message(
    frame: &mut ratatui::Frame,
    dialog_area: Rect,
    title: &str,
    message: &str,
    is_error: bool,
) {
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
