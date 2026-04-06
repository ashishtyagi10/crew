//! Simple terminal markdown renderer for AI responses.
//!
//! Supports: headings, bold, italic, inline code, code blocks, lists, and links.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const DEFAULT_BG: Color = Color::Indexed(234);

/// Parse markdown text into styled ratatui Lines (default background).
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    render_markdown_with_bg(text, DEFAULT_BG)
}

/// Parse markdown text into styled ratatui Lines with a custom background color.
pub fn render_markdown_with_bg(text: &str, bg: Color) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang;
    let mut table_rows: Vec<&str> = Vec::new();

    for raw_line in text.lines() {
        // Flush table buffer before code block toggle
        if raw_line.trim_start().starts_with("```") && !table_rows.is_empty() {
            lines.extend(render_table(&table_rows, bg));
            table_rows.clear();
        }

        // Code blocks
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block
                in_code_block = false;
                lines.push(Line::from(Span::styled(
                    " └──────────────────────────────────────────────────────────────────────────────",
                    Style::default().fg(Color::Indexed(240)).bg(bg),
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
                    format!("{:─<80}", header),
                    Style::default().fg(Color::Indexed(240)).bg(bg),
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

        // Flush table buffer if current line isn't a table row
        if !table_rows.is_empty() && !trimmed.starts_with('|') {
            lines.extend(render_table(&table_rows, bg));
            table_rows.clear();
        }

        // Table rows (collect for batch rendering)
        if trimmed.starts_with('|') {
            table_rows.push(trimmed);
            continue;
        }

        // Empty line
        if trimmed.is_empty() {
            lines.push(Line::from(Span::styled(" ", Style::default().bg(bg))));
            continue;
        }

        // Headings
        if trimmed.starts_with("### ") {
            let text = trimmed.trim_start_matches("### ");
            lines.push(Line::from(Span::styled(
                format!(" {}", text),
                Style::default()
                    .fg(Color::Cyan)
                    .bg(bg)
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
                    .bg(bg)
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
                    .bg(bg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(Line::from(Span::styled(
                " ─".repeat(24),
                Style::default().fg(Color::Indexed(240)).bg(bg),
            )));
            continue;
        }

        // Bullet lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let content = &trimmed[2..];
            let mut spans = vec![Span::styled(" • ", Style::default().fg(Color::Cyan).bg(bg))];
            spans.extend(parse_inline(content, bg));
            lines.push(Line::from(spans));
            continue;
        }

        // Numbered lists
        if let Some(rest) = strip_numbered_prefix(trimmed) {
            let mut spans = vec![Span::styled(
                format!(" {}", &trimmed[..trimmed.len() - rest.len()]),
                Style::default().fg(Color::Cyan).bg(bg),
            )];
            spans.extend(parse_inline(rest, bg));
            lines.push(Line::from(spans));
            continue;
        }

        // Regular paragraph with inline formatting
        let mut spans = vec![Span::styled(" ", Style::default().bg(bg))];
        spans.extend(parse_inline(trimmed, bg));
        lines.push(Line::from(spans));
    }

    // Flush remaining table rows
    if !table_rows.is_empty() {
        lines.extend(render_table(&table_rows, bg));
    }

    // Close unclosed code block
    if in_code_block {
        lines.push(Line::from(Span::styled(
            " └──────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(Color::Indexed(240)).bg(bg),
        )));
    }

    lines
}

/// Render a markdown table with box-drawing borders.
fn render_table(rows: &[&str], bg: Color) -> Vec<Line<'static>> {
    let table_bg = Color::Indexed(235);
    let border_style = Style::default().fg(Color::Indexed(240)).bg(bg);

    // Parse each row into cells
    let parsed: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let inner = row.trim_matches('|');
            inner
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    // Identify separator rows (e.g. |---|---|)
    let is_separator: Vec<bool> = rows
        .iter()
        .map(|row| {
            let inner = row.trim_matches('|');
            inner.split('|').all(|cell| {
                let t = cell.trim();
                !t.is_empty() && t.chars().all(|c| c == '-' || c == ':' || c == ' ')
            })
        })
        .collect();

    // Calculate column count and widths
    let num_cols = parsed.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return Vec::new();
    }

    let mut col_widths = vec![0usize; num_cols];
    for (i, cells) in parsed.iter().enumerate() {
        if is_separator[i] {
            continue;
        }
        for (j, cell) in cells.iter().enumerate() {
            if j < num_cols {
                col_widths[j] = col_widths[j].max(cell.len());
            }
        }
    }

    // Minimum column width
    for w in &mut col_widths {
        if *w < 3 {
            *w = 3;
        }
    }

    let mut lines = Vec::new();

    // Top border: ┌──┬──┐
    let top: String = col_widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┬");
    lines.push(Line::from(Span::styled(
        format!(" ┌{}┐", top),
        border_style,
    )));

    let mut after_header = false;
    for (i, cells) in parsed.iter().enumerate() {
        if is_separator[i] {
            // Separator: ├──┼──┤
            let sep: String = col_widths
                .iter()
                .map(|w| "─".repeat(w + 2))
                .collect::<Vec<_>>()
                .join("┼");
            lines.push(Line::from(Span::styled(
                format!(" ├{}┤", sep),
                border_style,
            )));
            after_header = true;
            continue;
        }

        let is_header = !after_header;
        let mut spans: Vec<Span<'static>> = vec![Span::styled(" │".to_string(), border_style)];

        for j in 0..num_cols {
            let cell_text = cells.get(j).map(|s| s.as_str()).unwrap_or("");
            let padded = format!(" {:<width$} ", cell_text, width = col_widths[j]);
            let cell_style = if is_header {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(table_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 200, 210)).bg(table_bg)
            };
            spans.push(Span::styled(padded, cell_style));
            spans.push(Span::styled("│".to_string(), border_style));
        }

        lines.push(Line::from(spans));
    }

    // Bottom border: └──┴──┘
    let bottom: String = col_widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┴");
    lines.push(Line::from(Span::styled(
        format!(" └{}┘", bottom),
        border_style,
    )));

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
fn parse_inline(text: &str, bg: Color) -> Vec<Span<'static>> {
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
