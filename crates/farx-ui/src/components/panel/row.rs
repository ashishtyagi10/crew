//! Painter for an individual entry row in the panel grid.

use farx_core::PanelState;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::entry_kind::{is_archive, is_executable, is_image};
use super::header::ColumnWidths;
use super::helpers::{format_size, pad_left, pad_right, truncate_or_pad};
use crate::theme::Theme;

pub(super) fn build_entry_line<'a>(
    panel: &'a PanelState,
    idx: usize,
    row_index: usize,
    cols: &ColumnWidths,
    theme: &'a Theme,
) -> Line<'a> {
    let entry = &panel.entries[idx];
    let is_cursor = idx == panel.cursor;
    let is_selected = panel.selected.contains(&idx);

    let row_bg = if row_index % 2 == 1 {
        theme.panel_bg_alt
    } else {
        theme.panel_bg
    };

    let marker_icon = if is_selected {
        "◆"
    } else if entry.is_dir {
        "▸"
    } else {
        " "
    };

    let name_display = truncate_or_pad(
        &format!("{}{}", marker_icon, entry.name),
        cols.name.saturating_sub(1),
    );

    let size_str = if entry.is_dir && (entry.name == ".." || entry.size == 0) {
        pad_left("<DIR>", cols.size)
    } else {
        pad_left(&format_size(entry.size), cols.size)
    };

    let date_str = match &entry.modified {
        Some(dt) => {
            let formatted = dt.format("%Y-%m-%d %H:%M").to_string();
            pad_right(&formatted, cols.date)
        }
        None => pad_right("", cols.date),
    };

    let entry_style = entry_style(entry, theme, row_bg, is_cursor, is_selected);

    let sep_style = if is_cursor {
        Style::default()
            .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
            .bg(entry_style.bg.unwrap_or(row_bg))
    } else {
        Style::default()
            .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
    };

    Line::from(vec![
        Span::styled(name_display, entry_style),
        Span::styled(theme.grid_separator, sep_style),
        Span::styled(size_str, entry_style),
        Span::styled(theme.grid_separator, sep_style),
        Span::styled(date_str, entry_style),
    ])
}

fn entry_style(
    entry: &farx_core::FileEntry,
    theme: &Theme,
    row_bg: ratatui::style::Color,
    is_cursor: bool,
    is_selected: bool,
) -> Style {
    if is_cursor && is_selected {
        theme.panel_cursor_selected
    } else if is_cursor {
        if entry.is_dir {
            theme.panel_cursor.add_modifier(Modifier::BOLD)
        } else {
            theme.panel_cursor
        }
    } else if is_selected {
        theme.panel_selected
    } else if entry.is_hidden && entry.name != ".." {
        Style::default()
            .fg(theme.panel_hidden.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
    } else if entry.is_dir {
        Style::default()
            .fg(theme.panel_dir.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    } else if is_image(entry) {
        Style::default()
            .fg(theme.panel_image.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
    } else if entry.is_symlink {
        Style::default()
            .fg(theme.panel_symlink.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
            .add_modifier(Modifier::ITALIC)
    } else if is_executable(entry) {
        Style::default()
            .fg(theme.panel_exe.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
    } else if is_archive(entry) {
        Style::default()
            .fg(theme.panel_archive.fg.unwrap_or(theme.panel_fg))
            .bg(row_bg)
    } else {
        Style::default().fg(theme.panel_fg).bg(row_bg)
    }
}
