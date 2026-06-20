use super::EditorState;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

pub(super) fn render(
    frame: &mut Frame,
    state: &mut EditorState,
    area: Rect,
    inner: Rect,
    visible_height: usize,
) {
    // Clamp preview scroll
    let total = state.preview_lines.len();
    if total > visible_height {
        state.preview_scroll = state
            .preview_scroll
            .min(total.saturating_sub(visible_height));
    } else {
        state.preview_scroll = 0;
    }
    let build_count = visible_height * 3;
    let end = (state.preview_scroll + build_count).min(total);
    let md_lines: Vec<Line> = state.preview_lines[state.preview_scroll..end].to_vec();
    let paragraph = Paragraph::new(md_lines).wrap(Wrap { trim: false });
    let text_area = Rect::new(inner.x, inner.y, inner.width, visible_height as u16);
    frame.render_widget(paragraph, text_area);

    // Scrollbar
    if total > visible_height {
        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(visible_height))
            .position(state.preview_scroll);
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

    // Status bar
    let status_y = inner.y + inner.height.saturating_sub(1);
    let pct = ((state.preview_scroll + visible_height).min(total) * 100)
        .checked_div(total)
        .unwrap_or(100);
    let status = format!(
        " MD Preview {}/{} ({}%) | Ctrl+M/Esc=Edit  PgUp/PgDn ",
        state.preview_scroll + 1,
        total,
        pct,
    );
    let status_line = Line::from(Span::styled(
        format!("{:<width$}", status, width = inner.width as usize),
        Style::default()
            .fg(Color::Rgb(16, 16, 18))
            .bg(Color::Rgb(100, 180, 220)),
    ));
    frame.render_widget(
        Paragraph::new(status_line),
        Rect::new(inner.x, status_y, inner.width, 1),
    );
}
