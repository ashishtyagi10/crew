use farx_core::PanelState;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Format a byte count into a human-readable size string.
pub fn format_size(size: u64) -> String {
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

    let header_height: u16 = 2; // header + separator line
    let footer_height: u16 = 1;
    let list_height = inner.height.saturating_sub(header_height + footer_height) as usize;
    let total_width = inner.width as usize;

    // Column widths
    let sep_w: usize = 1; // grid separator width
    let size_col_w: usize = 8;
    let date_col_w: usize = 16;
    let name_col_w: usize = total_width.saturating_sub(size_col_w + date_col_w + sep_w * 2);

    // ── Column header with grid separators ─────────────────────────
    let header_line = Line::from(vec![
        Span::styled(pad_right(" Name", name_col_w), theme.column_header),
        Span::styled(theme.grid_separator, theme.grid_style),
        Span::styled(pad_left("Size", size_col_w), theme.column_header),
        Span::styled(theme.grid_separator, theme.grid_style),
        Span::styled(pad_right("Modified", date_col_w), theme.column_header),
    ]);

    let header_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(header_line), header_area);

    // Separator line under header
    let sep_str = format!(
        "{}┼{}┼{}",
        "─".repeat(name_col_w),
        "─".repeat(size_col_w),
        "─".repeat(date_col_w),
    );
    let sep_line = Line::from(Span::styled(sep_str, theme.grid_style));
    let sep_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(sep_line), sep_area);

    // ── File list with zebra striping and grid lines ───────────────
    let list_area = Rect {
        x: inner.x,
        y: inner.y + header_height,
        width: inner.width,
        height: list_height as u16,
    };

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(list_height);
    let visible_end = (panel.scroll_offset + list_height).min(panel.entries.len());

    for idx in panel.scroll_offset..visible_end {
        let entry = &panel.entries[idx];
        let is_cursor = idx == panel.cursor;
        let is_selected = panel.selected.contains(&idx);
        let row_index = idx - panel.scroll_offset;

        // Zebra stripe background
        let row_bg = if row_index % 2 == 1 {
            theme.panel_bg_alt
        } else {
            theme.panel_bg
        };

        // File type / selection icon
        let marker_icon = if is_selected { "◆" } else if entry.is_dir { "▸" } else { " " };

        // Entry name
        let name_display = truncate_or_pad(
            &format!("{}{}", marker_icon, entry.name),
            name_col_w.saturating_sub(1),
        );

        // Size column
        let size_str = if entry.is_dir {
            pad_left("<DIR>", size_col_w)
        } else {
            pad_left(&format_size(entry.size), size_col_w)
        };

        // Modified column
        let date_str = match &entry.modified {
            Some(dt) => {
                let formatted = dt.format("%Y-%m-%d %H:%M").to_string();
                pad_right(&formatted, date_col_w)
            }
            None => pad_right("", date_col_w),
        };

        // Determine style based on state
        let entry_style = if is_cursor && is_selected {
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
            Style::default().fg(theme.panel_hidden.fg.unwrap_or(theme.panel_fg)).bg(row_bg)
        } else if entry.is_dir {
            Style::default()
                .fg(theme.panel_dir.fg.unwrap_or(theme.panel_fg))
                .bg(row_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_image(entry) {
            Style::default().fg(theme.panel_image.fg.unwrap_or(theme.panel_fg)).bg(row_bg)
        } else if entry.is_symlink {
            Style::default()
                .fg(theme.panel_symlink.fg.unwrap_or(theme.panel_fg))
                .bg(row_bg)
                .add_modifier(Modifier::ITALIC)
        } else if is_executable(entry) {
            Style::default().fg(theme.panel_exe.fg.unwrap_or(theme.panel_fg)).bg(row_bg)
        } else if is_archive(entry) {
            Style::default().fg(theme.panel_archive.fg.unwrap_or(theme.panel_fg)).bg(row_bg)
        } else {
            Style::default().fg(theme.panel_fg).bg(row_bg)
        };

        // Grid separator style — inherits cursor/selected bg when on that row
        let sep_style = if is_cursor {
            Style::default()
                .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
                .bg(entry_style.bg.unwrap_or(row_bg))
        } else {
            Style::default()
                .fg(theme.grid_style.fg.unwrap_or(theme.panel_fg))
                .bg(row_bg)
        };

        let line = Line::from(vec![
            Span::styled(name_display, entry_style),
            Span::styled(theme.grid_separator, sep_style),
            Span::styled(size_str, entry_style),
            Span::styled(theme.grid_separator, sep_style),
            Span::styled(date_str, entry_style),
        ]);

        lines.push(line);
    }

    // Fill remaining rows
    for i in lines.len()..list_height {
        let row_bg = if i % 2 == 1 {
            theme.panel_bg_alt
        } else {
            theme.panel_bg
        };
        lines.push(Line::from(Span::styled(
            " ".repeat(total_width),
            Style::default().bg(row_bg),
        )));
    }

    frame.render_widget(Paragraph::new(lines), list_area);

    // ── Footer ─────────────────────────────────────────────────────
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + header_height + list_height as u16,
        width: inner.width,
        height: footer_height,
    };

    let file_count = panel.entries.len();
    let selected_count = panel.selected.len();
    let selected_size: u64 = panel
        .selected
        .iter()
        .filter_map(|&i| panel.entries.get(i))
        .map(|e| e.size)
        .sum();

    let footer_text = if selected_count > 0 {
        format!(
            " {file_count} items | {selected_count} selected ({}) ",
            format_size(selected_size)
        )
    } else {
        format!(" {file_count} items ")
    };

    let footer_text = if let Some(ref qs) = panel.quick_search {
        format!("{footer_text}  {qs}")
    } else {
        footer_text
    };

    frame.render_widget(Paragraph::new(Line::from(Span::styled(footer_text, theme.footer))), footer_area);
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn is_executable(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some("sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl")
    )
}

fn is_archive(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some(
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst"
                | "tgz" | "tbz2" | "txz" | "lz" | "lzma" | "cab" | "iso"
                | "dmg" | "jar" | "war" | "deb" | "rpm"
        )
    )
}

fn is_image(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "heic")
    )
}

fn truncate_or_pad(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= width {
        format!("{s}{}", " ".repeat(width - w))
    } else {
        let mut result = String::new();
        let mut current_width = 0;
        for ch in s.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + ch_width >= width {
                break;
            }
            result.push(ch);
            current_width += ch_width;
        }
        while current_width < width.saturating_sub(1) {
            result.push(' ');
            current_width += 1;
        }
        result.push('~');
        result
    }
}

fn pad_right(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        s[..s.len().min(width)].to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

fn pad_left(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        s[..s.len().min(width)].to_string()
    } else {
        format!("{}{s}", " ".repeat(width - w))
    }
}
