use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::commands::SLASH_COMMANDS;
use super::state::SlashSuggestionsState;

/// Render the slash suggestion popup just above the command line area.
///
/// `cmd_area` is the Rect of the command line box — the popup floats above it.
pub fn render_slash_suggestions(frame: &mut Frame, state: &SlashSuggestionsState, cmd_area: Rect) {
    if state.matches.is_empty() {
        return;
    }

    let max_visible = 12u16;
    let item_count = state.matches.len() as u16;
    // +2 for top/bottom border
    let popup_height = item_count.min(max_visible) + 2;
    let popup_width = 48u16.min(cmd_area.width);

    // Position above the command line
    let y = cmd_area.y.saturating_sub(popup_height);
    let popup_area = Rect::new(cmd_area.x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commands ")
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Scroll the view so cursor is always visible
    let visible_rows = inner.height as usize;
    let scroll_offset = if state.cursor >= visible_rows {
        state.cursor - visible_rows + 1
    } else {
        0
    };

    for (row, &cmd_idx) in state
        .matches
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let cmd = &SLASH_COMMANDS[cmd_idx];
        let is_selected = scroll_offset + row == state.cursor;

        let (cmd_style, desc_style) = if is_selected {
            (
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Indexed(24))
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(Color::Indexed(250))
                    .bg(Color::Indexed(24)),
            )
        } else {
            (
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(236)),
            )
        };

        let available = inner.width as usize;
        let cmd_text = cmd.command;
        let desc_text = cmd.description;
        // Pad between command and description
        let gap = available
            .saturating_sub(cmd_text.len())
            .saturating_sub(desc_text.len())
            .saturating_sub(2); // 1 space prefix + 1 min gap

        let line = Line::from(vec![
            Span::styled(" ", cmd_style),
            Span::styled(cmd_text, cmd_style),
            Span::styled(" ".repeat(gap.max(1)), desc_style),
            Span::styled(desc_text, desc_style),
        ]);

        let row_area = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        // Fill background for selected row
        if is_selected {
            let bg_fill = " ".repeat(inner.width as usize);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    bg_fill,
                    Style::default().bg(Color::Indexed(24)),
                )),
                row_area,
            );
        }
        frame.render_widget(Paragraph::new(line), row_area);
    }
}
