//! Main per-line highlighter dispatch loop.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::colors::*;
use super::json::highlight_json_line;
use super::language::Language;
use super::markdown::highlight_markdown_line;
use super::scanners::{
    scan_decorator, scan_identifier, scan_lifetime, scan_number, scan_rust_attribute, scan_string,
};

/// Highlight a single line of code, returning owned styled spans.
pub fn highlight_line(line: &str, lang: Language, bg: Color) -> Vec<Span<'static>> {
    if lang == Language::Unknown {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_PLAIN).bg(bg),
        )];
    }

    let trimmed = line.trim_start();
    let comment_prefix = lang.comment_prefix();
    if !comment_prefix.is_empty() && trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_COMMENT)
                .bg(bg)
                .add_modifier(Modifier::ITALIC),
        )];
    }

    if lang == Language::Markdown {
        return highlight_markdown_line(line, bg);
    }
    if lang == Language::Json {
        return highlight_json_line(line, bg);
    }

    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        if ch == '"'
            || ch == '\''
            || (ch == '`' && matches!(lang, Language::JavaScript | Language::TypeScript))
        {
            spans.push(scan_string(line, &mut chars, i, ch, bg));
            continue;
        }

        if ch == '/'
            && matches!(
                lang,
                Language::Rust
                    | Language::Go
                    | Language::C
                    | Language::Cpp
                    | Language::Java
                    | Language::JavaScript
                    | Language::TypeScript
                    | Language::Css
            )
            && line[i..].starts_with("//")
        {
            spans.push(Span::styled(
                line[i..].to_string(),
                Style::default()
                    .fg(C_COMMENT)
                    .bg(bg)
                    .add_modifier(Modifier::ITALIC),
            ));
            return spans;
        }
        if ch == '#'
            && matches!(
                lang,
                Language::Python
                    | Language::Ruby
                    | Language::Shell
                    | Language::Toml
                    | Language::Yaml
            )
        {
            spans.push(Span::styled(
                line[i..].to_string(),
                Style::default()
                    .fg(C_COMMENT)
                    .bg(bg)
                    .add_modifier(Modifier::ITALIC),
            ));
            return spans;
        }

        if ch.is_ascii_digit()
            && (i == 0
                || !line
                    .as_bytes()
                    .get(i.wrapping_sub(1))
                    .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .unwrap_or(false))
        {
            chars.next();
            spans.push(scan_number(line, &mut chars, i, bg));
            continue;
        }

        // Rust lifetimes: 'a, 'static (falls back to char literal-ish plain).
        if ch == '\'' && lang == Language::Rust {
            if let Some(span) = scan_lifetime(line, &mut chars, i, bg) {
                spans.push(span);
            } else {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(C_PLAIN).bg(bg),
                ));
            }
            continue;
        }

        // Decorators: @decorator (Python/Java/TS).
        if ch == '@'
            && matches!(
                lang,
                Language::Python | Language::Java | Language::TypeScript
            )
        {
            spans.push(scan_decorator(line, &mut chars, i, bg));
            continue;
        }

        // Rust attributes: #[...]
        if ch == '#' && lang == Language::Rust {
            spans.push(scan_rust_attribute(line, &mut chars, i, bg));
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            spans.push(scan_identifier(line, &mut chars, i, lang, bg));
            continue;
        }

        if "=+-*/<>!&|^%~?:".contains(ch) {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(C_OPERATOR).bg(bg),
            ));
            chars.next();
            continue;
        }

        if "{}[]();,.@".contains(ch) {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(C_PUNCT).bg(bg),
            ));
            chars.next();
            continue;
        }

        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(C_PLAIN).bg(bg),
        ));
        chars.next();
    }

    spans
}
