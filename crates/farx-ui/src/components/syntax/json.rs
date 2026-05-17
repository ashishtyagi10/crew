//! JSON line highlighter.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::colors::*;

pub(super) fn highlight_json_line(line: &str, bg: Color) -> Vec<Span<'static>> {
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.contains("\":") {
        if let Some(colon_pos) = line.find(':') {
            let key_part = &line[..colon_pos + 1];
            let val_part = &line[colon_pos + 1..];
            let val_trimmed = val_part.trim();
            let val_color = if val_trimmed.starts_with('"') {
                C_STRING
            } else if val_trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
                C_NUMBER
            } else if val_trimmed == "true"
                || val_trimmed == "false"
                || val_trimmed == "null"
                || val_trimmed == "true,"
                || val_trimmed == "false,"
                || val_trimmed == "null,"
            {
                C_KEYWORD
            } else {
                C_PLAIN
            };
            return vec![
                Span::styled(key_part.to_string(), Style::default().fg(C_TYPE).bg(bg)),
                Span::styled(val_part.to_string(), Style::default().fg(val_color).bg(bg)),
            ];
        }
    }
    if trimmed.starts_with('"') {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_STRING).bg(bg),
        )];
    }
    vec![Span::styled(
        line.to_string(),
        Style::default().fg(C_PUNCT).bg(bg),
    )]
}
