//! Simple terminal markdown renderer for AI responses.
//!
//! Supports: headings, bold, italic, inline code, code blocks, lists, and links.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const BG: Color = Color::Indexed(234);

/// Parse markdown text into styled ratatui Lines.
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang;

    for raw_line in text.lines() {
        // Code blocks
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block
                in_code_block = false;
                lines.push(Line::from(Span::styled(
                    " └─────────────────────────────────────────────",
                    Style::default().fg(Color::Indexed(240)).bg(BG),
                )));
            } else {
                // Start code block
                in_code_block = true;
                code_lang = raw_line.trim_start().trim_start_matches('`').to_string();
                let header = if code_lang.is_empty() {
                    " ┌─ code ".to_string()
                } else {
                    format!(" ┌─ {} ", code_lang)
                };
                lines.push(Line::from(Span::styled(
                    format!("{:─<48}", header),
                    Style::default().fg(Color::Indexed(240)).bg(BG),
                )));
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!(" │ {}", raw_line),
                Style::default().fg(Color::Green).bg(Color::Indexed(235)),
            )));
            continue;
        }

        let trimmed = raw_line.trim();

        // Empty line
        if trimmed.is_empty() {
            lines.push(Line::from(Span::styled(" ", Style::default().bg(BG))));
            continue;
        }

        // Headings
        if trimmed.starts_with("### ") {
            let text = trimmed.trim_start_matches("### ");
            lines.push(Line::from(Span::styled(
                format!(" {}", text),
                Style::default()
                    .fg(Color::Cyan)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if trimmed.starts_with("## ") {
            let text = trimmed.trim_start_matches("## ");
            lines.push(Line::from(Span::styled(
                format!(" {}", text),
                Style::default()
                    .fg(Color::Yellow)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if trimmed.starts_with("# ") {
            let text = trimmed.trim_start_matches("# ");
            lines.push(Line::from(Span::styled(
                format!(" {}", text),
                Style::default()
                    .fg(Color::Yellow)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(Line::from(Span::styled(
                " ─".repeat(24),
                Style::default().fg(Color::Indexed(240)).bg(BG),
            )));
            continue;
        }

        // Bullet lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let content = &trimmed[2..];
            let mut spans = vec![Span::styled(
                " • ",
                Style::default().fg(Color::Cyan).bg(BG),
            )];
            spans.extend(parse_inline(content));
            lines.push(Line::from(spans));
            continue;
        }

        // Numbered lists
        if let Some(rest) = strip_numbered_prefix(trimmed) {
            let mut spans = vec![Span::styled(
                format!(
                    " {}",
                    &trimmed[..trimmed.len() - rest.len()]
                ),
                Style::default().fg(Color::Cyan).bg(BG),
            )];
            spans.extend(parse_inline(rest));
            lines.push(Line::from(spans));
            continue;
        }

        // Regular paragraph with inline formatting
        let mut spans = vec![Span::styled(" ", Style::default().bg(BG))];
        spans.extend(parse_inline(trimmed));
        lines.push(Line::from(spans));
    }

    // Close unclosed code block
    if in_code_block {
        lines.push(Line::from(Span::styled(
            " └─────────────────────────────────────────────",
            Style::default().fg(Color::Indexed(240)).bg(BG),
        )));
    }

    lines
}

/// Strip a numbered list prefix like "1. ", "2. ", etc. Returns the rest of the line.
fn strip_numbered_prefix(s: &str) -> Option<&str> {
    let mut chars = s.chars();
    // Must start with digit
    if !chars.next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return None;
    }
    // Consume remaining digits
    let rest = chars.as_str();
    let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    // Must be followed by ". "
    after_digits.strip_prefix(". ")
}

/// Parse inline markdown formatting: **bold**, *italic*, `code`, [links](url)
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Bold: **text**
        if let Some(pos) = remaining.find("**") {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
            }
            remaining = &remaining[pos + 2..];
            if let Some(end) = remaining.find("**") {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default()
                        .fg(Color::White)
                        .bg(BG)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = &remaining[end + 2..];
                continue;
            } else {
                spans.push(Span::styled(
                    "**".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
                continue;
            }
        }

        // Inline code: `code`
        if let Some(pos) = remaining.find('`') {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
            }
            remaining = &remaining[pos + 1..];
            if let Some(end) = remaining.find('`') {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Indexed(236)),
                ));
                remaining = &remaining[end + 1..];
                continue;
            } else {
                spans.push(Span::styled(
                    "`".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
                continue;
            }
        }

        // Italic: *text* (only if not **)
        if let Some(pos) = remaining.find('*') {
            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
            }
            remaining = &remaining[pos + 1..];
            if let Some(end) = remaining.find('*') {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default()
                        .fg(Color::Rgb(135, 215, 255))
                        .bg(BG)
                        .add_modifier(Modifier::ITALIC),
                ));
                remaining = &remaining[end + 1..];
                continue;
            } else {
                spans.push(Span::styled(
                    "*".to_string(),
                    Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
                ));
                continue;
            }
        }

        // No more formatting — emit rest as plain text
        spans.push(Span::styled(
            remaining.to_string(),
            Style::default().fg(Color::Rgb(135, 215, 255)).bg(BG),
        ));
        break;
    }

    spans
}
