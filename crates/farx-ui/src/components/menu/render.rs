use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::{MenuAction, MenuState};
use crate::theme::Theme;

pub fn render_menu(frame: &mut Frame, state: &MenuState, _theme: &Theme) {
    let area = frame.area();

    // Menu bar at top (1 line)
    let bar_area = Rect::new(area.x, area.y, area.width, 1);
    let bar_bg = Style::default().fg(Color::Black).bg(Color::Cyan);

    // Build menu bar line
    let mut spans = Vec::new();
    let mut x_positions: Vec<u16> = Vec::new();
    let mut x = 0u16;

    for (i, menu) in state.menus.iter().enumerate() {
        x_positions.push(x);
        let style = if i == state.active_menu {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            bar_bg
        };
        let label = menu.title;
        spans.push(Span::styled(label, style));
        x += label.len() as u16;
    }
    // Fill rest of bar
    let remaining = area.width.saturating_sub(x) as usize;
    spans.push(Span::styled(" ".repeat(remaining), bar_bg));

    frame.render_widget(Paragraph::new(Line::from(spans)), bar_area);

    // Draw dropdown for active menu
    if state.dropdown_open && state.active_menu < state.menus.len() {
        render_dropdown(frame, state, x_positions[state.active_menu], area);
    }
}

fn render_dropdown(frame: &mut Frame, state: &MenuState, dropdown_x: u16, area: Rect) {
    let menu = &state.menus[state.active_menu];

    // Calculate dropdown width
    let max_label = menu
        .items
        .iter()
        .map(|i| i.label.len() + i.hotkey.len() + 4)
        .max()
        .unwrap_or(20);
    let dropdown_width = (max_label as u16 + 2).min(area.width - dropdown_x);
    let dropdown_height = menu.items.len() as u16 + 2; // +2 for borders

    let dropdown_area = Rect::new(
        dropdown_x,
        1, // right below the menu bar
        dropdown_width,
        dropdown_height.min(area.height.saturating_sub(1)),
    );

    frame.render_widget(Clear, dropdown_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)));

    let inner = block.inner(dropdown_area);
    frame.render_widget(block, dropdown_area);

    for (i, item) in menu.items.iter().enumerate() {
        if i >= inner.height as usize {
            break;
        }

        let is_separator = item.action == MenuAction::None;
        let is_selected = i == state.active_item;

        let item_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);

        if is_separator {
            let sep = "\u{2500}".repeat(inner.width as usize);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    sep,
                    Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
                )),
                item_area,
            );
        } else {
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Indexed(24))
            } else {
                Style::default().fg(Color::White).bg(Color::Indexed(236))
            };
            let hotkey_style = if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::Indexed(24))
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
            };

            let padding = inner.width as usize
                - item.label.len().min(inner.width as usize)
                - item.hotkey.len().min(inner.width as usize);
            let line = Line::from(vec![
                Span::styled(item.label, style),
                Span::styled(" ".repeat(padding.max(1)), style),
                Span::styled(item.hotkey, hotkey_style),
            ]);
            frame.render_widget(Paragraph::new(line), item_area);
        }
    }
}
