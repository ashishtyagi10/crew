use super::format::{format_permissions, format_size};
use crate::theme::Theme;
use farx_core::tree::{GitFileStatus, TreeState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Build a single row line for the tree panel at index `idx` in `tree.visible_nodes`.
/// `row_index` is the on-screen row (used for zebra striping).
pub(super) fn build_row_line<'a>(
    tree: &TreeState,
    theme: &Theme,
    idx: usize,
    row_index: usize,
    total_width: usize,
) -> Line<'a> {
    let node = &tree.visible_nodes[idx];
    let is_cursor = idx == tree.cursor;
    let is_selected = tree.selected.contains(&idx);

    let row_bg = if row_index % 2 == 1 {
        theme.panel_bg_alt
    } else {
        theme.panel_bg
    };

    // Tree indent with guide lines
    let indent: String = (0..node.depth).map(|_| "  │").collect::<String>();
    let connector = if node.depth > 0 { "── " } else { " " };

    let icon = if node.entry.is_dir {
        if node.expanded {
            "[-] "
        } else if node.has_children {
            "[+] "
        } else {
            "[ ] "
        }
    } else if is_selected {
        "◆ "
    } else {
        "· "
    };

    let name = &node.entry.name;
    let symlink_target = if node.entry.is_symlink {
        std::fs::read_link(&node.entry.path)
            .ok()
            .map(|t| format!(" → {}", t.display()))
            .unwrap_or_else(|| " → ?".to_string())
    } else {
        String::new()
    };

    let size_col: String = if node.entry.is_dir {
        format!("{:>7}", "<DIR>")
    } else {
        format!("{:>7}", format_size(node.entry.size))
    };
    let perms_col: String = node
        .entry
        .mode
        .map(|m| format!(" {}", format_permissions(m)))
        .unwrap_or_else(|| " ".repeat(10));
    let date_col: String = node
        .entry
        .modified
        .map(|m| format!(" {}", m.format("%m-%d %H:%M")))
        .unwrap_or_else(|| " ".repeat(12));

    let git_indicator = tree.git_status_for(&node.entry.path);
    let (git_glyph, git_color) = match git_indicator {
        Some(GitFileStatus::Modified) => (" M", Color::Rgb(230, 140, 70)),
        Some(GitFileStatus::Staged) => (" S", Color::Rgb(120, 190, 90)),
        Some(GitFileStatus::Untracked) => (" ?", Color::Rgb(150, 150, 150)),
        Some(GitFileStatus::Conflict) => (" !", Color::Rgb(240, 80, 80)),
        Some(GitFileStatus::Deleted) => (" D", Color::Rgb(240, 80, 80)),
        Some(GitFileStatus::Renamed) => (" R", Color::Rgb(140, 180, 250)),
        Some(GitFileStatus::Ignored) => ("", Color::Reset),
        None => ("", Color::Reset),
    };

    let meta_width = 7 + 10 + 12 + if git_glyph.is_empty() { 0 } else { 2 };
    let prefix_len = indent.chars().count() + connector.chars().count() + icon.chars().count();
    let name_width = total_width.saturating_sub(prefix_len + meta_width);

    let name_display = format!("{}{}", name, symlink_target);
    let name_padded = if name_display.chars().count() >= name_width {
        let truncated: String = name_display
            .chars()
            .take(name_width.saturating_sub(1))
            .collect();
        format!("{}~", truncated)
    } else {
        format!("{:<width$}", name_display, width = name_width)
    };

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

    let guide_style = if is_cursor {
        Style::default()
            .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
            .bg(entry_style.bg.unwrap_or(row_bg))
    } else {
        Style::default().fg(Color::Rgb(60, 60, 65)).bg(row_bg)
    };

    let icon_style = if is_cursor || is_selected {
        entry_style
    } else if node.entry.is_dir {
        Style::default().fg(Color::Rgb(180, 150, 60)).bg(row_bg)
    } else {
        Style::default().fg(Color::Rgb(80, 80, 85)).bg(row_bg)
    };

    let meta_style = if is_cursor || is_selected {
        entry_style
    } else {
        Style::default().fg(Color::Indexed(245)).bg(row_bg)
    };

    let size_style = if is_cursor || is_selected {
        entry_style
    } else if node.entry.is_dir {
        Style::default().fg(Color::Indexed(242)).bg(row_bg)
    } else {
        Style::default().fg(Color::Indexed(248)).bg(row_bg)
    };

    let mut spans = vec![
        Span::styled(indent.clone(), guide_style),
        Span::styled(connector.to_string(), guide_style),
        Span::styled(icon.to_string(), icon_style),
        Span::styled(name_padded, entry_style),
        Span::styled(size_col, size_style),
        Span::styled(perms_col, meta_style),
        Span::styled(date_col, meta_style),
    ];
    if !git_glyph.is_empty() {
        let git_style = if is_cursor {
            Style::default()
                .fg(git_color)
                .bg(entry_style.bg.unwrap_or(row_bg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(git_color)
                .bg(row_bg)
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(git_glyph.to_string(), git_style));
    }
    Line::from(spans)
}
