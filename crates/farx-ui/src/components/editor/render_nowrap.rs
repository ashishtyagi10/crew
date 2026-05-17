use super::{EditorMode, EditorState};
use crate::components::syntax::{highlight_line, Language};
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub(super) fn render(
    frame: &mut Frame,
    state: &EditorState,
    area: Rect,
    inner: Rect,
    visible_height: usize,
    gutter_width: u16,
    text_width: usize,
) {
    let ext = state.file_path.extension().and_then(|e| e.to_str());
    let lang = Language::from_extension(ext);
    let bg_normal = Color::Rgb(22, 22, 26);

    let mut text_lines: Vec<Line> = Vec::new();
    for i in state.scroll_offset..(state.scroll_offset + visible_height).min(state.lines.len()) {
        let line_num = format!("{:>5} ", i + 1);
        let line = &state.lines[i];

        let visible_text: String = line
            .chars()
            .skip(state.horizontal_scroll)
            .take(text_width)
            .collect();

        let is_cursor_line = i == state.cursor_line;
        let bg = if is_cursor_line {
            Color::Indexed(236)
        } else {
            bg_normal
        };
        let line_num_style = Style::default().fg(Color::DarkGray).bg(bg);

        let mut spans = vec![Span::styled(line_num, line_num_style)];

        let highlighted = highlight_line(&visible_text, lang, bg);
        if highlighted.is_empty() {
            spans.push(Span::styled(
                format!("{:<width$}", visible_text, width = text_width),
                Style::default().fg(Color::Rgb(200, 200, 210)).bg(bg),
            ));
        } else {
            spans.extend(highlighted);
            let used: usize = visible_text.len();
            if used < text_width {
                spans.push(Span::styled(
                    " ".repeat(text_width - used),
                    Style::default().bg(bg),
                ));
            }
        }

        text_lines.push(Line::from(spans));
    }

    for _ in text_lines.len()..visible_height {
        text_lines.push(Line::from(vec![
            Span::styled("    ~ ", Style::default().fg(Color::DarkGray).bg(bg_normal)),
            Span::styled(" ".repeat(text_width), Style::default().bg(bg_normal)),
        ]));
    }

    let text_area = Rect::new(inner.x, inner.y, inner.width, visible_height as u16);
    frame.render_widget(Paragraph::new(text_lines), text_area);

    if state.lines.len() > visible_height {
        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        let mut scrollbar_state =
            ScrollbarState::new(state.lines.len().saturating_sub(visible_height))
                .position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(Color::Rgb(50, 50, 55)))
                .thumb_style(Style::default().fg(Color::Rgb(120, 120, 140))),
            scrollbar_area,
            &mut scrollbar_state,
        );
    }

    if state.mode == EditorMode::Normal {
        let visual_col = state.cursor_col.saturating_sub(state.horizontal_scroll) as u16;
        let cursor_x = inner.x + gutter_width + visual_col;
        let cursor_y = inner.y + (state.cursor_line.saturating_sub(state.scroll_offset)) as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + visible_height as u16 {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
