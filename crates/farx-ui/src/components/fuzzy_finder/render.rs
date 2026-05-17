use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::state::FuzzyFinderState;
use crate::theme::Theme;

pub fn render_fuzzy_finder(frame: &mut Frame, state: &FuzzyFinderState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " Find File (Ctrl+P) — {} files ",
            state.all_files.len()
        ))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Query input
    let query_display = format!(
        " {:<width$}",
        state.query,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            query_display,
            Style::default().fg(Color::White).bg(Color::Indexed(238)),
        ))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.set_cursor_position((inner.x + 1 + state.cursor_pos as u16, inner.y));

    // Results count
    let count_line = Line::from(Span::styled(
        format!(" {} matches", state.results.len()),
        Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
    ));
    frame.render_widget(
        Paragraph::new(count_line),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Results list
    let list_start = inner.y + 2;
    let visible = (inner.height.saturating_sub(3)) as usize;

    let scroll = if state.result_cursor >= state.result_scroll + visible {
        state.result_cursor - visible + 1
    } else if state.result_cursor < state.result_scroll {
        state.result_cursor
    } else {
        state.result_scroll
    };

    for (i, result) in state.results.iter().skip(scroll).take(visible).enumerate() {
        let is_selected = scroll + i == state.result_cursor;
        let style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(24))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let icon = if result.path.is_dir() { "[D] " } else { "    " };
        let display = format!(" {}{}", icon, result.rel_path);
        let truncated: String = display.chars().take(inner.width as usize).collect();
        frame.render_widget(
            Paragraph::new(Span::styled(truncated, style)),
            Rect::new(inner.x, list_start + i as u16, inner.width, 1),
        );
    }

    // Hint
    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Enter=Go  Up/Down=Navigate  Esc=Close",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
