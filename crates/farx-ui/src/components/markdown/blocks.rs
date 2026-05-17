//! Block-level markdown rendering: headings, lists, code blocks, horizontal rules.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::inline::parse_inline;

/// Push the opening fence of a code block. Returns the captured language.
pub(super) fn push_code_block_open(lines: &mut Vec<Line<'static>>, raw_line: &str, bg: Color) {
    let code_lang = raw_line.trim_start().trim_start_matches('`').to_string();
    let header = if code_lang.is_empty() {
        " ┌─ code ".to_string()
    } else {
        format!(" ┌─ {} ", code_lang)
    };
    lines.push(Line::from(Span::styled(
        format!("{:─<80}", header),
        Style::default().fg(Color::Indexed(240)).bg(bg),
    )));
}

/// Push the closing fence of a code block.
pub(super) fn push_code_block_close(lines: &mut Vec<Line<'static>>, bg: Color) {
    lines.push(Line::from(Span::styled(
        " └──────────────────────────────────────────────────────────────────────────────",
        Style::default().fg(Color::Indexed(240)).bg(bg),
    )));
}

/// Push a line of code-block content.
pub(super) fn push_code_block_line(lines: &mut Vec<Line<'static>>, raw_line: &str) {
    lines.push(Line::from(Span::styled(
        format!(" │ {}", raw_line),
        Style::default().fg(Color::Green).bg(Color::Indexed(235)),
    )));
}

/// Try to render heading/hr/list. Returns true if line was rendered.
pub(super) fn try_render_block(lines: &mut Vec<Line<'static>>, trimmed: &str, bg: Color) -> bool {
    // Headings
    if let Some(text) = trimmed.strip_prefix("### ") {
        lines.push(Line::from(Span::styled(
            format!(" {}", text),
            Style::default()
                .fg(Color::Cyan)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )));
        return true;
    }
    if let Some(text) = trimmed.strip_prefix("## ") {
        lines.push(Line::from(Span::styled(
            format!(" {}", text),
            Style::default()
                .fg(Color::Yellow)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )));
        return true;
    }
    if let Some(text) = trimmed.strip_prefix("# ") {
        lines.push(Line::from(Span::styled(
            format!(" {}", text),
            Style::default()
                .fg(Color::Yellow)
                .bg(bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        return true;
    }

    // Horizontal rule
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        lines.push(Line::from(Span::styled(
            " ─".repeat(24),
            Style::default().fg(Color::Indexed(240)).bg(bg),
        )));
        return true;
    }

    // Bullet lists
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let content = &trimmed[2..];
        let mut spans = vec![Span::styled(" • ", Style::default().fg(Color::Cyan).bg(bg))];
        spans.extend(parse_inline(content, bg));
        lines.push(Line::from(spans));
        return true;
    }

    // Numbered lists
    if let Some(rest) = strip_numbered_prefix(trimmed) {
        let mut spans = vec![Span::styled(
            format!(" {}", &trimmed[..trimmed.len() - rest.len()]),
            Style::default().fg(Color::Cyan).bg(bg),
        )];
        spans.extend(parse_inline(rest, bg));
        lines.push(Line::from(spans));
        return true;
    }

    false
}

/// Render a paragraph line with inline formatting.
pub(super) fn push_paragraph(lines: &mut Vec<Line<'static>>, trimmed: &str, bg: Color) {
    let mut spans = vec![Span::styled(" ", Style::default().bg(bg))];
    spans.extend(parse_inline(trimmed, bg));
    lines.push(Line::from(spans));
}

/// Push an empty line with the appropriate background.
pub(super) fn push_empty(lines: &mut Vec<Line<'static>>, bg: Color) {
    lines.push(Line::from(Span::styled(" ", Style::default().bg(bg))));
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
