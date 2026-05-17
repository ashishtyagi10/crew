use ratatui::prelude::*;

use super::helpers::{get_disk_space, is_image_ext};
use super::image_preview::render_image_preview;

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
        let file_preview = current_file.and_then(build_file_preview);

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

fn build_file_preview(entry: &farx_core::FileEntry) -> Option<FilePreview> {
    if entry.name == ".." {
        return None;
    }
    let modified = entry
        .modified
        .map(|m| m.format("%Y-%m-%d %H:%M:%S").to_string());

    let content_lines = read_content_lines(entry);

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
}

fn read_content_lines(entry: &farx_core::FileEntry) -> Vec<String> {
    if entry.is_dir {
        // For directories, show child count
        return match std::fs::read_dir(&entry.path) {
            Ok(rd) => vec![format!("{} entries", rd.count())],
            Err(_) => vec!["(cannot read)".to_string()],
        };
    }
    if entry.size > 5 * 1024 * 1024 {
        return vec!["(file too large to preview)".to_string()];
    }
    // Try reading as text
    match std::fs::read(&entry.path) {
        Ok(bytes) => {
            let check = &bytes[..bytes.len().min(512)];
            if check.contains(&0) {
                // Binary file — show hex summary
                let mut lines = vec![format!("Binary file ({} bytes)", bytes.len())];
                for chunk in bytes.chunks(16).take(8) {
                    let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
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
}
