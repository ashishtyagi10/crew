use std::path::Path;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

pub struct InfoPanelData {
    pub current_dir: String,
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_size: u64,
    pub selected_count: usize,
    pub selected_size: u64,
    pub free_space: Option<u64>,
    pub total_space: Option<u64>,
}

impl InfoPanelData {
    pub fn from_panel(panel: &farx_core::PanelState) -> Self {
        let mut total_files = 0;
        let mut total_dirs = 0;
        let mut total_size = 0u64;
        let mut selected_size = 0u64;

        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == ".." {
                continue;
            }
            if entry.is_dir {
                total_dirs += 1;
            } else {
                total_files += 1;
                total_size += entry.size;
            }
            if panel.selected.contains(&i) {
                selected_size += entry.size;
            }
        }

        // Get disk space info
        let (free_space, total_space) = get_disk_space(&panel.current_dir);

        Self {
            current_dir: panel.current_dir.display().to_string(),
            total_files,
            total_dirs,
            total_size,
            selected_count: panel.selected.len(),
            selected_size,
            free_space,
            total_space,
        }
    }
}

fn get_disk_space(_path: &Path) -> (Option<u64>, Option<u64>) {
    // Platform-specific disk space query
    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path = CString::new(_path.to_string_lossy().as_bytes()).ok();
        if let Some(c_path) = c_path {
            unsafe {
                let mut stat: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                    let free = stat.f_bavail as u64 * stat.f_frsize;
                    let total = stat.f_blocks as u64 * stat.f_frsize;
                    return (Some(free), Some(total));
                }
            }
        }
        (None, None)
    }
    #[cfg(not(unix))]
    {
        (None, None)
    }
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn render_info_panel(frame: &mut Frame, area: Rect, data: &InfoPanelData, _theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Info ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Rgb(200, 200, 210)).bg(Color::Rgb(22, 22, 26)))
        .style(Style::default().bg(Color::Rgb(22, 22, 26)).fg(Color::Rgb(200, 200, 210)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    let label_style = Style::default().fg(Color::Yellow).bg(Color::Rgb(22, 22, 26));
    let value_style = Style::default().fg(Color::White).bg(Color::Rgb(22, 22, 26));
    let dim_style = Style::default().fg(Color::Rgb(200, 200, 210)).bg(Color::Rgb(22, 22, 26));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Directory Info",
        label_style.add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  Files:       ", label_style),
        Span::styled(format!("{}", data.total_files), value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Directories: ", label_style),
        Span::styled(format!("{}", data.total_dirs), value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Total size:  ", label_style),
        Span::styled(format_size(data.total_size), value_style),
    ]));

    if data.selected_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Selection",
            label_style.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Selected:    ", label_style),
            Span::styled(format!("{} file(s)", data.selected_count), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Sel. size:   ", label_style),
            Span::styled(format_size(data.selected_size), value_style),
        ]));
    }

    if data.free_space.is_some() || data.total_space.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Disk",
            label_style.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        if let Some(total) = data.total_space {
            lines.push(Line::from(vec![
                Span::styled("  Total:       ", label_style),
                Span::styled(format_size(total), value_style),
            ]));
        }
        if let Some(free) = data.free_space {
            lines.push(Line::from(vec![
                Span::styled("  Free:        ", label_style),
                Span::styled(format_size(free), value_style),
            ]));
        }
        if let (Some(free), Some(total)) = (data.free_space, data.total_space) {
            if total > 0 {
                let used_pct = ((total - free) as f64 / total as f64 * 100.0) as u64;
                lines.push(Line::from(vec![
                    Span::styled("  Used:        ", label_style),
                    Span::styled(format!("{}%", used_pct), value_style),
                ]));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Ctrl+L to close",
        dim_style,
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}
