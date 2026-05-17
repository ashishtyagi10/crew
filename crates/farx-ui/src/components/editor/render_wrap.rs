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

    let mut visual_lines: Vec<Line> = Vec::new();
    let mut cursor_visual_y: Option<u16> = None;
    let mut cursor_visual_x: Option<u16> = None;
    let mut logical_idx = state.scroll_offset;

    while visual_lines.len() < visible_height && logical_idx < state.lines.len() {
        let line = &state.lines[logical_idx];
        let is_cursor_line = logical_idx == state.cursor_line;
        let bg = if is_cursor_line {
            Color::Indexed(236)
        } else {
            bg_normal
        };
        let line_num_style = Style::default().fg(Color::DarkGray).bg(bg);

        let chars: Vec<char> = line.chars().collect();
        let chunk_count = if chars.is_empty() {
            1
        } else {
            chars.len().div_ceil(text_width)
        };

        for chunk_idx in 0..chunk_count {
            if visual_lines.len() >= visible_height {
                break;
            }
            let char_start = chunk_idx * text_width;
            let char_end = (char_start + text_width).min(chars.len());
            let chunk_text: String = chars[char_start..char_end].iter().collect();

            let gutter = if chunk_idx == 0 {
                format!("{:>5} ", logical_idx + 1)
            } else {
                "    > ".to_string()
            };
            let mut spans = vec![Span::styled(gutter, line_num_style)];

            let highlighted = highlight_line(&chunk_text, lang, bg);
            if highlighted.is_empty() {
                spans.push(Span::styled(
                    format!("{:<width$}", chunk_text, width = text_width),
                    Style::default().fg(Color::Rgb(200, 200, 210)).bg(bg),
                ));
            } else {
                spans.extend(highlighted);
                let used = chunk_text.len();
                if used < text_width {
                    spans.push(Span::styled(
                        " ".repeat(text_width - used),
                        Style::default().bg(bg),
                    ));
                }
            }

            if is_cursor_line {
                let cursor_char_col = line[..state.cursor_col.min(line.len())].chars().count();
                if cursor_char_col >= char_start && cursor_char_col <= char_end {
                    cursor_visual_y = Some(visual_lines.len() as u16);
                    cursor_visual_x = Some((cursor_char_col - char_start) as u16);
                }
            }

            visual_lines.push(Line::from(spans));
        }
        logical_idx += 1;
    }

    while visual_lines.len() < visible_height {
        visual_lines.push(Line::from(vec![
            Span::styled("    ~ ", Style::default().fg(Color::DarkGray).bg(bg_normal)),
            Span::styled(" ".repeat(text_width), Style::default().bg(bg_normal)),
        ]));
    }

    let text_area = Rect::new(inner.x, inner.y, inner.width, visible_height as u16);
    frame.render_widget(Paragraph::new(visual_lines), text_area);

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
        if let (Some(vy), Some(vx)) = (cursor_visual_y, cursor_visual_x) {
            let cx = inner.x + gutter_width + vx;
            let cy = inner.y + vy;
            if cx < inner.x + inner.width && cy < inner.y + visible_height as u16 {
                frame.set_cursor_position((cx, cy));
            }
        }
    }
}
