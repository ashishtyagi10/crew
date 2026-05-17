//! Token scanners used by the per-line highlighter. Each scanner consumes
//! characters from a peekable `CharIndices` iterator and returns one styled
//! `Span`. The `start` index is the byte offset of the first character
//! already peeked (but not yet consumed) at call time.

use std::iter::Peekable;
use std::str::CharIndices;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::colors::*;
use super::language::Language;

pub(super) type Chars<'a> = Peekable<CharIndices<'a>>;

fn end_index(chars: &mut Chars<'_>, line: &str) -> usize {
    chars.peek().map(|&(i, _)| i).unwrap_or(line.len())
}

/// Scan a quoted string literal (", ', or backtick for JS/TS).
pub(super) fn scan_string(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    quote: char,
    bg: Color,
) -> Span<'static> {
    chars.next();
    let mut escaped = false;
    while let Some(&(_, c)) = chars.peek() {
        chars.next();
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            break;
        }
    }
    let end = end_index(chars, line);
    Span::styled(
        line[start..end].to_string(),
        Style::default().fg(C_STRING).bg(bg),
    )
}

/// Scan a numeric literal (decimal, hex, binary, octal).
pub(super) fn scan_number(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    bg: Color,
) -> Span<'static> {
    while let Some(&(_, c)) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == 'x' || c == 'b' || c == 'o' {
            chars.next();
        } else {
            break;
        }
    }
    let end = end_index(chars, line);
    Span::styled(
        line[start..end].to_string(),
        Style::default().fg(C_NUMBER).bg(bg),
    )
}

/// Scan a Rust lifetime (`'a`, `'static`). Returns None if it turned out to
/// be a char literal (single quote with no trailing word chars).
pub(super) fn scan_lifetime(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    bg: Color,
) -> Option<Span<'static>> {
    chars.next();
    while let Some(&(_, c)) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '_' {
            chars.next();
        } else {
            break;
        }
    }
    let end = end_index(chars, line);
    if end > start + 1 {
        Some(Span::styled(
            line[start..end].to_string(),
            Style::default().fg(C_LIFETIME).bg(bg),
        ))
    } else {
        None
    }
}

/// Scan a decorator/annotation like `@decorator` (Python/Java/TS).
pub(super) fn scan_decorator(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    bg: Color,
) -> Span<'static> {
    chars.next();
    while let Some(&(_, c)) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            chars.next();
        } else {
            break;
        }
    }
    let end = end_index(chars, line);
    Span::styled(
        line[start..end].to_string(),
        Style::default().fg(C_MACRO).bg(bg),
    )
}

/// Scan a Rust attribute `#[...]`.
pub(super) fn scan_rust_attribute(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    bg: Color,
) -> Span<'static> {
    while let Some(&(_, c)) = chars.peek() {
        chars.next();
        if c == ']' {
            break;
        }
    }
    let end = end_index(chars, line);
    Span::styled(
        line[start..end].to_string(),
        Style::default().fg(C_MACRO).bg(bg),
    )
}

/// Scan an identifier word and classify it based on language tables.
pub(super) fn scan_identifier(
    line: &str,
    chars: &mut Chars<'_>,
    start: usize,
    lang: Language,
    bg: Color,
) -> Span<'static> {
    while let Some(&(_, c)) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '!' || c == '?' {
            chars.next();
        } else {
            break;
        }
    }
    let end = end_index(chars, line);
    let word = &line[start..end];
    let bare = word.trim_end_matches('!');

    let style = if lang.control_flow().contains(&bare) {
        Style::default()
            .fg(C_CONTROL)
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    } else if lang.special_idents().contains(&bare) {
        Style::default()
            .fg(C_SPECIAL)
            .bg(bg)
            .add_modifier(Modifier::ITALIC)
    } else if lang.keywords().contains(&bare) {
        Style::default()
            .fg(C_KEYWORD)
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    } else if lang.builtins().contains(&word) {
        Style::default().fg(C_BUILTIN).bg(bg)
    } else if word.ends_with('!') && lang == Language::Rust {
        Style::default().fg(C_MACRO).bg(bg)
    } else if word.starts_with("__") && word.ends_with("__") {
        Style::default().fg(C_MACRO).bg(bg)
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        Style::default().fg(C_TYPE).bg(bg)
    } else if chars.peek().map(|&(_, c)| c == '(').unwrap_or(false) {
        Style::default().fg(C_FUNC).bg(bg)
    } else {
        Style::default().fg(C_PLAIN).bg(bg)
    };
    Span::styled(word.to_string(), style)
}
