mod breadcrumb;
mod filter_bar;
mod footer;
mod format;
mod row;
mod tab_bar;

pub use breadcrumb::breadcrumb_path_at_click;
pub use tab_bar::render_tab_bar;

use crate::theme::Theme;
use farx_core::tree::TreeState;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render_tree_panel(
    frame: &mut Frame,
    area: Rect,
    tree: &TreeState,
    is_active: bool,
    theme: &Theme,
) {
    render_tree_panel_with_filter(frame, area, tree, is_active, theme, false);
}

pub fn render_tree_panel_with_filter(
    frame: &mut Frame,
    area: Rect,
    tree: &TreeState,
    is_active: bool,
    theme: &Theme,
    filter_editing: bool,
) {
    let border_style = if is_active {
        theme.panel_border_active
    } else {
        theme.panel_border
    };

    let title_line = breadcrumb::build_breadcrumb_title(
        &tree.root,
        area.width.saturating_sub(4),
        is_active,
        theme,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title_line)
        .style(Style::default().bg(theme.panel_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let filter_height = filter_bar::filter_height(tree, filter_editing);
    if filter_height > 0 {
        filter_bar::render_filter_bar(frame, inner, tree, filter_editing);
    }

    let footer_height: u16 = 1;
    let list_height = inner.height.saturating_sub(footer_height + filter_height) as usize;
    let total_width = inner.width as usize;

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(list_height);
    let visible_end = (tree.scroll_offset + list_height).min(tree.visible_nodes.len());

    for idx in tree.scroll_offset..visible_end {
        let row_index = idx - tree.scroll_offset;
        lines.push(row::build_row_line(
            tree,
            theme,
            idx,
            row_index,
            total_width,
        ));
    }

    // Fill empty rows
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

    let list_area = Rect {
        x: inner.x,
        y: inner.y + filter_height,
        width: inner.width,
        height: list_height as u16,
    };
    frame.render_widget(Paragraph::new(lines), list_area);

    let footer_area = Rect {
        x: inner.x,
        y: inner.y + filter_height + list_height as u16,
        width: inner.width,
        height: footer_height,
    };
    footer::render_footer(frame, footer_area, tree, theme);
}
