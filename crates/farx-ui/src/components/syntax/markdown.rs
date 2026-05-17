//! Markdown line highlighter.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::colors::*;

pub(super) fn highlight_markdown_line(line: &str, bg: Color) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_KEYWORD)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )];
    }
    if trimmed.starts_with("```") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_COMMENT).bg(bg),
        )];
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_TYPE).bg(bg),
        )];
    }
    if trimmed.starts_with("> ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_STRING)
                .bg(bg)
                .add_modifier(Modifier::ITALIC),
        )];
    }
    vec![Span::styled(
        line.to_string(),
        Style::default().fg(C_PLAIN).bg(bg),
    )]
}
