use ratatui::prelude::*;

use super::data::InfoPanelData;
use super::helpers::format_size;
use super::render::SectionStyles;

pub(super) fn append_dir_section(
    lines: &mut Vec<Line<'static>>,
    data: &InfoPanelData,
    styles: &SectionStyles,
) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Directory Info",
        styles.label.add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  Files:       ", styles.label),
        Span::styled(format!("{}", data.total_files), styles.value),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Directories: ", styles.label),
        Span::styled(format!("{}", data.total_dirs), styles.value),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Total size:  ", styles.label),
        Span::styled(format_size(data.total_size), styles.value),
    ]));

    if data.selected_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Selection",
            styles.label.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Selected:    ", styles.label),
            Span::styled(format!("{} file(s)", data.selected_count), styles.value),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Sel. size:   ", styles.label),
            Span::styled(format_size(data.selected_size), styles.value),
        ]));
    }
}

pub(super) fn append_disk_section(
    lines: &mut Vec<Line<'static>>,
    data: &InfoPanelData,
    styles: &SectionStyles,
) {
    if data.free_space.is_none() && data.total_space.is_none() {
        return;
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Disk",
        styles.label.add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    if let Some(total) = data.total_space {
        lines.push(Line::from(vec![
            Span::styled("  Total:       ", styles.label),
            Span::styled(format_size(total), styles.value),
        ]));
    }
    if let Some(free) = data.free_space {
        lines.push(Line::from(vec![
            Span::styled("  Free:        ", styles.label),
            Span::styled(format_size(free), styles.value),
        ]));
    }
    if let (Some(free), Some(total)) = (data.free_space, data.total_space) {
        if total > 0 {
            let used_pct = ((total - free) as f64 / total as f64 * 100.0) as u64;
            lines.push(Line::from(vec![
                Span::styled("  Used:        ", styles.label),
                Span::styled(format!("{}%", used_pct), styles.value),
            ]));
        }
    }
}

pub(super) fn append_preview_section(
    lines: &mut Vec<Line<'static>>,
    data: &InfoPanelData,
    styles: &SectionStyles,
    inner: Rect,
) {
    let Some(ref preview) = data.file_preview else {
        return;
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  File Preview",
        styles.label.add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Name:        ", styles.label),
        Span::styled(preview.name.clone(), styles.value),
    ]));
    if !preview.is_dir {
        lines.push(Line::from(vec![
            Span::styled("  Size:        ", styles.label),
            Span::styled(format_size(preview.size), styles.value),
        ]));
    }
    if preview.is_symlink {
        lines.push(Line::from(vec![
            Span::styled("  Type:        ", styles.label),
            Span::styled("symlink", styles.value),
        ]));
    }
    if let Some(ref modified) = preview.modified {
        lines.push(Line::from(vec![
            Span::styled("  Modified:    ", styles.label),
            Span::styled(modified.clone(), styles.value),
        ]));
    }

    // Image preview (if available)
    if let Some((w, h)) = preview.image_dimensions {
        lines.push(Line::from(vec![
            Span::styled("  Dimensions:  ", styles.label),
            Span::styled(format!("{}×{}", w, h), styles.value),
        ]));
    }
    if !preview.image_lines.is_empty() {
        lines.push(Line::from(""));
        let max_image = (inner.height as usize).saturating_sub(lines.len() + 2);
        for img_line in preview.image_lines.iter().take(max_image) {
            lines.push(img_line.clone());
        }
    } else if !preview.content_lines.is_empty() {
        // Content preview (text/hex)
        lines.push(Line::from(""));
        let max_content = (inner.height as usize).saturating_sub(lines.len() + 2);
        let content_style = Style::default()
            .fg(Color::Rgb(170, 170, 180))
            .bg(Color::Rgb(22, 22, 26));
        for line in preview.content_lines.iter().take(max_content) {
            let display: String = line.chars().take(inner.width as usize - 2).collect();
            lines.push(Line::from(Span::styled(
                format!("  {}", display),
                content_style,
            )));
        }
        if preview.content_lines.len() > max_content {
            lines.push(Line::from(Span::styled("  ...", styles.dim)));
        }
    }
}
