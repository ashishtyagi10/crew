use crate::theme::Theme;
use farx_core::tree::TreeState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
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
    let dir_display = tree.root.to_string_lossy().to_string();
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
        .title(Span::styled(format!(" Tree: {dir_display} "), title_style))
        .style(Style::default().bg(theme.panel_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    // Filter bar (shown when filter is active)
    let filter_height: u16 = if !tree.filter.is_empty() || filter_editing {
        1
    } else {
        0
    };
    if filter_height > 0 {
        let filter_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        };
        let filter_display = format!(
            " Filter: {:<width$}",
            tree.filter,
            width = (inner.width as usize).saturating_sub(10)
        );
        let filter_style = if filter_editing {
            Style::default().fg(Color::Yellow).bg(Color::Indexed(238))
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(237))
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(filter_display, filter_style))),
            filter_area,
        );
        if filter_editing {
            frame.set_cursor_position((inner.x + 9 + tree.filter.len() as u16, inner.y));
        }
    }

    let footer_height: u16 = 1;
    let list_height = inner.height.saturating_sub(footer_height + filter_height) as usize;
    let total_width = inner.width as usize;

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(list_height);
    let visible_end = (tree.scroll_offset + list_height).min(tree.visible_nodes.len());

    for idx in tree.scroll_offset..visible_end {
        let node = &tree.visible_nodes[idx];
        let is_cursor = idx == tree.cursor;
        let is_selected = tree.selected.contains(&idx);
        let row_index = idx - tree.scroll_offset;

        let row_bg = if row_index % 2 == 1 {
            theme.panel_bg_alt
        } else {
            theme.panel_bg
        };

        // Tree indent with guide lines
        let indent: String = (0..node.depth).map(|_| "  │").collect::<String>();
        let connector = if node.depth > 0 { "── " } else { " " };

        // Icon — using standard Unicode that works in every terminal
        let icon = if node.entry.is_dir {
            if node.expanded {
                "[-] " // expanded
            } else if node.has_children {
                "[+] " // collapsed with children
            } else {
                "[ ] " // empty dir
            }
        } else if is_selected {
            "◆ "
        } else {
            "· " // simple dot for files
        };

        let name = &node.entry.name;
        let size_str = if node.entry.is_dir {
            String::new()
        } else {
            format!("  {}", format_size(node.entry.size))
        };

        let name_and_size = format!("{}{}", name, size_str);

        // Styles
        let entry_style = if is_cursor && is_selected {
            theme.panel_cursor_selected
        } else if is_cursor {
            if node.entry.is_dir {
                theme.panel_cursor.add_modifier(Modifier::BOLD)
            } else {
                theme.panel_cursor
            }
        } else if is_selected {
            theme.panel_selected
        } else if node.entry.is_dir {
            Style::default()
                .fg(theme.panel_dir.fg.unwrap_or(theme.panel_fg))
                .bg(row_bg)
                .add_modifier(Modifier::BOLD)
        } else if node.entry.is_hidden {
            Style::default()
                .fg(theme.panel_hidden.fg.unwrap_or(theme.panel_fg))
                .bg(row_bg)
        } else {
            Style::default().fg(theme.panel_fg).bg(row_bg)
        };

        // Guide line style — dim, always visible
        let guide_style = if is_cursor {
            Style::default()
                .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
                .bg(entry_style.bg.unwrap_or(row_bg))
        } else {
            Style::default().fg(Color::Rgb(60, 60, 65)).bg(row_bg)
        };

        // Icon style — slightly different color than name
        let icon_style = if is_cursor || is_selected {
            entry_style
        } else if node.entry.is_dir {
            Style::default().fg(Color::Rgb(180, 150, 60)).bg(row_bg)
        } else {
            Style::default().fg(Color::Rgb(80, 80, 85)).bg(row_bg)
        };

        // Calculate how much space the name part gets
        let prefix_len = indent.chars().count() + connector.chars().count() + icon.chars().count();
        let name_width = total_width.saturating_sub(prefix_len);
        let name_padded = if name_and_size.len() >= name_width {
            format!("{}~", &name_and_size[..name_width.saturating_sub(1)])
        } else {
            format!("{:<width$}", name_and_size, width = name_width)
        };

        lines.push(Line::from(vec![
            Span::styled(indent.clone(), guide_style),
            Span::styled(connector.to_string(), guide_style),
            Span::styled(icon.to_string(), icon_style),
            Span::styled(name_padded, entry_style),
        ]));
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

    // Footer
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + filter_height + list_height as u16,
        width: inner.width,
        height: footer_height,
    };

    let node_count = tree.visible_nodes.len();
    let selected_count = tree.selected.len();
    let footer_text = if selected_count > 0 {
        format!("  {} items | {} selected", node_count, selected_count)
    } else {
        format!("  {} items", node_count)
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer_text, theme.footer))),
        footer_area,
    );
}

fn format_size(size: u64) -> String {
    if size < 1_000 {
        format!("{size} B")
    } else if size < 1_000_000 {
        format!("{:.1}K", size as f64 / 1_024.0)
    } else if size < 1_000_000_000 {
        format!("{:.1}M", size as f64 / 1_048_576.0)
    } else {
        format!("{:.1}G", size as f64 / 1_073_741_824.0)
    }
}
