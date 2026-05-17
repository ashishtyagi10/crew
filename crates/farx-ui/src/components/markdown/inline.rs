//! Inline markdown formatting: **bold**, *italic*, `code`.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Parse inline markdown formatting: **bold**, *italic*, `code`, [links](url)
pub(super) fn parse_inline(text: &str, bg: Color) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Bold: **text**
        if let Some(pos) = remaining.find("**") {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
            }
            remaining = &remaining[pos + 2..];
            if let Some(end) = remaining.find("**") {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default()
                        .fg(Color::White)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = &remaining[end + 2..];
                continue;
            } else {
                spans.push(Span::styled(
                    "**".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
                continue;
            }
        }

        // Inline code: `code`
        if let Some(pos) = remaining.find('`') {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
            }
            remaining = &remaining[pos + 1..];
            if let Some(end) = remaining.find('`') {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default().fg(Color::Green).bg(Color::Indexed(236)),
                ));
                remaining = &remaining[end + 1..];
                continue;
            } else {
                spans.push(Span::styled(
                    "`".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
                continue;
            }
        }

        // Italic: *text* (only if not **)
        if let Some(pos) = remaining.find('*') {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
            }
            remaining = &remaining[pos + 1..];
            if let Some(end) = remaining.find('*') {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default()
                        .fg(Color::Rgb(135, 215, 255))
                        .bg(bg)
                        .add_modifier(Modifier::ITALIC),
                ));
                remaining = &remaining[end + 1..];
                continue;
            } else {
                spans.push(Span::styled(
                    "*".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
                ));
                continue;
            }
        }

        // No more formatting — emit rest as plain text
        spans.push(Span::styled(
            remaining.to_string(),
            Style::default().fg(Color::Rgb(135, 215, 255)).bg(bg),
        ));
        break;
    }

    spans
}
