use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::state::BookmarkState;
use crate::theme::Theme;

pub fn render_bookmarks(frame: &mut Frame, state: &BookmarkState, _theme: &Theme) {
    let area = frame.area();

    let dialog_width = 60u16.min(area.width.saturating_sub(4));
    let dialog_height = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Bookmarks (Ctrl+B) ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if state.bookmarks.is_empty() {
        let msg = Line::from(Span::styled(
            " No bookmarks. Press Alt+B to bookmark current directory.",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(
            Paragraph::new(msg),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
    } else {
        let visible = (inner.height.saturating_sub(2)) as usize;
        let scroll = if state.cursor >= state.scroll + visible {
            state.cursor - visible + 1
        } else if state.cursor < state.scroll {
            state.cursor
        } else {
            state.scroll
        };

        for (i, bm) in state
            .bookmarks
            .iter()
            .skip(scroll)
            .take(visible)
            .enumerate()
        {
            let is_selected = scroll + i == state.cursor;
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Indexed(24))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
            };

            let display = format!(" {:<name_w$} {}", bm.name, bm.path.display(), name_w = 12);
            let truncated: String = display.chars().take(inner.width as usize).collect();
            frame.render_widget(
                Paragraph::new(Span::styled(truncated, style)),
                Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
            );
        }
    }

    // Hint bar
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint = " Enter=Go  Del/F8=Remove  Esc=Close";
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
