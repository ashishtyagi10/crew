//! Top-level orchestration of panel rendering: block, header, list, footer.

use farx_core::PanelState;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::footer::render_footer;
use super::header::{render_header, ColumnWidths};
use super::row::build_entry_line;
use crate::theme::Theme;

/// Render a single file panel (left or right) inside the given area.
pub fn render_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    theme: &Theme,
) {
    let dir_display = panel.current_dir.to_string_lossy().to_string();
    let border_style = if is_active {
        theme.panel_border_active
    } else {
        theme.panel_border
    };

    let title_style = if is_active {
        Style::default()
            .fg(theme.panel_header_fg)
            .bg(theme.panel_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.panel_header_fg)
            .bg(theme.panel_bg)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(format!(" {dir_display} "), title_style))
        .style(Style::default().bg(theme.panel_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let header_height: u16 = 2;
    let footer_height: u16 = 1;
    let list_height = inner.height.saturating_sub(header_height + footer_height) as usize;
    let total_width = inner.width as usize;
    let cols = ColumnWidths::compute(total_width);

    render_header(frame, inner, &cols, theme);

    let list_area = Rect {
        x: inner.x,
        y: inner.y + header_height,
        width: inner.width,
        height: list_height as u16,
    };

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(list_height);
    let visible_end = (panel.scroll_offset + list_height).min(panel.entries.len());

    for idx in panel.scroll_offset..visible_end {
        let row_index = idx - panel.scroll_offset;
        lines.push(build_entry_line(panel, idx, row_index, &cols, theme));
    }

    for i in lines.len()..list_height {
        let bg = if i % 2 == 1 {
            theme.panel_bg_alt
        } else {
            theme.panel_bg
        };
        lines.push(Line::from(Span::styled(
            " ".repeat(total_width),
            Style::default().bg(bg),
        )));
    }

    frame.render_widget(Paragraph::new(lines), list_area);

    let footer_area = Rect {
        x: inner.x,
        y: inner.y + header_height + list_height as u16,
        width: inner.width,
        height: footer_height,
    };
    render_footer(frame, footer_area, panel, theme);
}
