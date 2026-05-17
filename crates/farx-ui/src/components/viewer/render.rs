use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::components::syntax::{highlight_line, Language};
use crate::theme::Theme;

use super::hex::format_file_size;
use super::render_status::render_status_bar;
use super::state::ViewerState;

pub fn render_viewer(frame: &mut Frame, state: &mut ViewerState, _theme: &Theme) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    // Title with file info
    let file_name = state
        .file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mode_str = if state.hex_mode {
        " [HEX]"
    } else if state.markdown_mode {
        " [MD]"
    } else {
        ""
    };
    let title = format!(
        " {} - {}{} ",
        file_name,
        format_file_size(state.file_size),
        mode_str
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Rgb(200, 200, 210))
                .bg(Color::Rgb(22, 22, 26)),
        )
        .style(
            Style::default()
                .bg(Color::Rgb(22, 22, 26))
                .fg(Color::Rgb(200, 200, 210)),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve 1 row for the status bar (rendered over bottom border)
    let content_height = inner.height as usize;
    state.visible_height = content_height;

    // Re-clamp scroll offset after visible_height is known
    if state.total_lines > content_height {
        state.scroll_offset = state
            .scroll_offset
            .min(state.total_lines.saturating_sub(content_height));
    } else {
        state.scroll_offset = 0;
    }

    let text_lines = build_text_lines(state, content_height);

    let paragraph = if state.wrap {
        Paragraph::new(text_lines).wrap(Wrap { trim: false })
    } else {
        Paragraph::new(text_lines)
    };

    frame.render_widget(paragraph, inner);

    // Scrollbar
    if state.total_lines > content_height {
        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        let mut scrollbar_state =
            ScrollbarState::new(state.total_lines.saturating_sub(content_height))
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

    render_status_bar(frame, area, state, content_height);
}

fn build_text_lines(state: &ViewerState, content_height: usize) -> Vec<Line<'static>> {
    let bg = Color::Rgb(22, 22, 26);

    // When wrap is on, build extra lines to fill viewport (wrapped lines consume
    // more visual space). A 3x multiplier ensures coverage even with long lines.
    let build_count = if state.wrap {
        content_height * 3
    } else {
        content_height
    };

    if state.markdown_mode {
        // Markdown preview: use pre-rendered lines
        let end = (state.scroll_offset + build_count).min(state.markdown_lines.len());
        state.markdown_lines[state.scroll_offset..end].to_vec()
    } else {
        // Detect language from file extension
        let ext = state.file_path.extension().and_then(|e| e.to_str());
        let lang = Language::from_extension(ext);

        let mut lines: Vec<Line<'static>> = Vec::new();
        for i in state.scroll_offset..(state.scroll_offset + build_count).min(state.total_lines) {
            if i < state.lines.len() {
                let line_num = format!("{:>6} ", i + 1);
                let content = state.lines[i].clone();

                let mut spans = vec![Span::styled(
                    line_num,
                    Style::default().fg(Color::DarkGray).bg(bg),
                )];

                if state.hex_mode {
                    spans.push(Span::styled(
                        content,
                        Style::default().fg(Color::Rgb(200, 200, 210)).bg(bg),
                    ));
                } else {
                    spans.extend(highlight_line(&state.lines[i], lang, bg));
                }

                lines.push(Line::from(spans));
            }
        }
        lines
    }
}
