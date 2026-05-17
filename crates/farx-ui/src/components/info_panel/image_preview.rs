use std::path::Path;

use ratatui::prelude::*;

/// Render an image as half-block character lines.
/// Returns (dimensions, rendered_lines).
pub fn render_image_preview(
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
