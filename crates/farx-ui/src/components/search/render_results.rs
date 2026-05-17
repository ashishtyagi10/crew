use super::state::SearchState;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub(crate) fn render_results_and_hint(
    frame: &mut Frame,
    state: &SearchState,
    inner: Rect,
    mut y_offset: u16,
) {
    // Results area
    let results_height = inner.height.saturating_sub(y_offset + 1);

    if state.searching {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Searching...",
                Style::default().fg(Color::Yellow).bg(Color::Indexed(236)),
            )),
            Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
        );
    } else if !state.results.is_empty() {
        // Show result count
        let count_line = Line::from(Span::styled(
            format!(" Found {} file(s):", state.results.len()),
            Style::default().fg(Color::Green).bg(Color::Indexed(236)),
        ));
        frame.render_widget(
            Paragraph::new(count_line),
            Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
        );
        y_offset += 1;

        render_result_list(frame, state, inner, y_offset, results_height);
    }

    render_hint(frame, state, inner);
}

fn render_result_list(
    frame: &mut Frame,
    state: &SearchState,
    inner: Rect,
    y_offset: u16,
    results_height: u16,
) {
    let visible = (results_height.saturating_sub(1)) as usize;
    // Adjust scroll
    let scroll = if state.result_cursor >= state.result_scroll + visible {
        state.result_cursor - visible + 1
    } else if state.result_cursor < state.result_scroll {
        state.result_cursor
    } else {
        state.result_scroll
    };

    let mut row = 0usize;
    let mut result_idx = scroll;
    while row < visible && result_idx < state.results.len() {
        let result = &state.results[result_idx];
        let is_selected = result_idx == state.result_cursor;
        let file_style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Indexed(24))
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let prefix = if result.is_dir { "[DIR] " } else { "      " };
        let display = format!(" {}{}", prefix, result.path.display());
        let truncated: String = display.chars().take(inner.width as usize).collect();

        frame.render_widget(
            Paragraph::new(Span::styled(truncated, file_style)),
            Rect::new(inner.x, inner.y + y_offset + row as u16, inner.width, 1),
        );
        row += 1;

        // Show matching lines if content search was used
        if !result.matching_lines.is_empty() {
            let match_style = if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::Indexed(24))
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
            };
            for (line_num, line_text) in &result.matching_lines {
                if row >= visible {
                    break;
                }
                let match_display = format!("        {}:{}", line_num, line_text);
                let match_truncated: String =
                    match_display.chars().take(inner.width as usize).collect();
                frame.render_widget(
                    Paragraph::new(Span::styled(match_truncated, match_style)),
                    Rect::new(inner.x, inner.y + y_offset + row as u16, inner.width, 1),
                );
                row += 1;
            }
        }
        result_idx += 1;
    }
}

fn render_hint(frame: &mut Frame, state: &SearchState, inner: Rect) {
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint = if state.results.is_empty() {
        " Enter=Search  Tab=Switch field  Esc=Close"
    } else {
        " Enter=Go to  Up/Down=Navigate  Tab=New search  Esc=Close"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
