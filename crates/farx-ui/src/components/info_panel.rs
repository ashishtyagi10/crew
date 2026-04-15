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
    /// Preview data for the file under cursor
    pub file_preview: Option<FilePreview>,
}

pub struct FilePreview {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<String>,
    /// First N lines of text content, or hex summary for binary
    pub content_lines: Vec<String>,
    /// Image dimensions if this is an image file.
    pub image_dimensions: Option<(u32, u32)>,
    /// Pre-rendered image preview lines using half-block characters.
    pub image_lines: Vec<Line<'static>>,
}

impl InfoPanelData {
    pub fn from_panel(
        panel: &farx_core::PanelState,
        current_file: Option<&farx_core::FileEntry>,
    ) -> Self {
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

        // Build file preview for current entry
        let file_preview = current_file.and_then(|entry| {
            if entry.name == ".." {
                return None;
            }
            let modified = entry
                .modified
                .map(|m| m.format("%Y-%m-%d %H:%M:%S").to_string());

            let content_lines = if entry.is_dir {
                // For directories, show child count
                match std::fs::read_dir(&entry.path) {
                    Ok(rd) => vec![format!("{} entries", rd.count())],
                    Err(_) => vec!["(cannot read)".to_string()],
                }
            } else if entry.size > 5 * 1024 * 1024 {
                vec!["(file too large to preview)".to_string()]
            } else {
                // Try reading as text
                match std::fs::read(&entry.path) {
                    Ok(bytes) => {
                        let check = &bytes[..bytes.len().min(512)];
                        if check.contains(&0) {
                            // Binary file — show hex summary
                            let mut lines = vec![format!("Binary file ({} bytes)", bytes.len())];
                            for chunk in bytes.chunks(16).take(8) {
                                let hex: Vec<String> =
                                    chunk.iter().map(|b| format!("{:02x}", b)).collect();
                                lines.push(hex.join(" "));
                            }
                            if bytes.len() > 128 {
                                lines.push("...".to_string());
                            }
                            lines
                        } else {
                            // Text file — show first 30 lines
                            let text = String::from_utf8_lossy(&bytes);
                            text.lines().take(30).map(|l| l.to_string()).collect()
                        }
                    }
                    Err(e) => vec![format!("(error: {})", e)],
                }
            };

            // Check if this is an image file
            let (image_dimensions, image_lines) = if is_image_ext(&entry.name) {
                render_image_preview(&entry.path, 40, 20)
            } else {
                (None, Vec::new())
            };

            Some(FilePreview {
                name: entry.name.clone(),
                size: entry.size,
                is_dir: entry.is_dir,
                is_symlink: entry.is_symlink,
                modified,
                content_lines,
                image_dimensions,
                image_lines,
            })
        });

        Self {
            current_dir: panel.current_dir.display().to_string(),
            total_files,
            total_dirs,
            total_size,
            selected_count: panel.selected.len(),
            selected_size,
            free_space,
            total_space,
            file_preview,
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
        .border_style(
            Style::default()
                .fg(Color::Rgb(200, 200, 210))
                .bg(Color::Rgb(22, 22, 26)),
        )
        .style(
            Style::default()
                .bg(Color::Rgb(22, 22, 26))
                .fg(Color::Rgb(200, 200, 210)),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    let label_style = Style::default()
        .fg(Color::Yellow)
        .bg(Color::Rgb(22, 22, 26));
    let value_style = Style::default().fg(Color::White).bg(Color::Rgb(22, 22, 26));
    let dim_style = Style::default()
        .fg(Color::Rgb(200, 200, 210))
        .bg(Color::Rgb(22, 22, 26));

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

    // File preview section
    if let Some(ref preview) = data.file_preview {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  File Preview",
            label_style.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Name:        ", label_style),
            Span::styled(&preview.name, value_style),
        ]));
        if !preview.is_dir {
            lines.push(Line::from(vec![
                Span::styled("  Size:        ", label_style),
                Span::styled(format_size(preview.size), value_style),
            ]));
        }
        if preview.is_symlink {
            lines.push(Line::from(vec![
                Span::styled("  Type:        ", label_style),
                Span::styled("symlink", value_style),
            ]));
        }
        if let Some(ref modified) = preview.modified {
            lines.push(Line::from(vec![
                Span::styled("  Modified:    ", label_style),
                Span::styled(modified.as_str(), value_style),
            ]));
        }

        // Image preview (if available)
        if let Some((w, h)) = preview.image_dimensions {
            lines.push(Line::from(vec![
                Span::styled("  Dimensions:  ", label_style),
                Span::styled(format!("{}×{}", w, h), value_style),
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
                lines.push(Line::from(Span::styled("  ...", dim_style)));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Ctrl+L to close", dim_style)));

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Check if a filename has an image extension.
fn is_image_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
        || lower.ends_with(".ico")
        || lower.ends_with(".tiff")
        || lower.ends_with(".tif")
}

/// Render an image as half-block character lines.
/// Returns (dimensions, rendered_lines).
fn render_image_preview(
    path: &Path,
    max_width: u32,
    max_height: u32,
) -> (Option<(u32, u32)>, Vec<Line<'static>>) {
    let img = match image::open(path) {
        Ok(img) => img,
        Err(_) => return (None, Vec::new()),
    };

    let (orig_w, orig_h) = (img.width(), img.height());

    // Scale to fit: each character cell is 1 wide, but uses ▀ to show 2 pixel rows
    let target_w = max_width;
    let target_h = max_height * 2; // 2 pixel rows per character row

    let scaled = img.thumbnail(target_w, target_h).to_rgba8();
    let (sw, sh) = (scaled.width(), scaled.height());

    let mut lines = Vec::new();

    // Process 2 rows of pixels at a time using ▀ (upper half block)
    // fg = top pixel color, bg = bottom pixel color
    let mut y = 0u32;
    while y < sh {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::raw("  ")); // indent

        for x in 0..sw {
            let top = scaled.get_pixel(x, y);
            let bottom = if y + 1 < sh {
                scaled.get_pixel(x, y + 1)
            } else {
                &image::Rgba([22, 22, 26, 255]) // match panel bg
            };

            let fg = Color::Rgb(top[0], top[1], top[2]);
            let bg = Color::Rgb(bottom[0], bottom[1], bottom[2]);

            spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
        }

        lines.push(Line::from(spans));
        y += 2;
    }

    (Some((orig_w, orig_h)), lines)
}
