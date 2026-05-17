//! String formatting + padding helpers used by the panel renderer.

use unicode_width::UnicodeWidthStr;

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

pub(super) fn truncate_or_pad(s: &str, width: usize) -> String {
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

pub(super) fn pad_right(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        s[..s.len().min(width)].to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

pub(super) fn pad_left(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        s[..s.len().min(width)].to_string()
    } else {
        format!("{}{s}", " ".repeat(width - w))
    }
}
