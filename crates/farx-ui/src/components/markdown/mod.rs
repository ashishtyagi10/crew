//! Simple terminal markdown renderer for AI responses.
//!
//! Supports: headings, bold, italic, inline code, code blocks, lists, and links.

use ratatui::style::Color;
use ratatui::text::Line;

mod blocks;
mod inline;
mod table;

use blocks::{
    push_code_block_close, push_code_block_line, push_code_block_open, push_empty, push_paragraph,
    try_render_block,
};
use table::render_table;

const DEFAULT_BG: Color = Color::Indexed(234);

/// Parse markdown text into styled ratatui Lines (default background).
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    render_markdown_with_bg(text, DEFAULT_BG)
}

/// Parse markdown text into styled ratatui Lines with a custom background color.
pub fn render_markdown_with_bg(text: &str, bg: Color) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
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
                in_code_block = false;
                push_code_block_close(&mut lines, bg);
            } else {
                in_code_block = true;
                push_code_block_open(&mut lines, raw_line, bg);
            }
            continue;
        }

        if in_code_block {
            push_code_block_line(&mut lines, raw_line);
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
            push_empty(&mut lines, bg);
            continue;
        }

        // Headings, hr, lists
        if try_render_block(&mut lines, trimmed, bg) {
            continue;
        }

        // Regular paragraph with inline formatting
        push_paragraph(&mut lines, trimmed, bg);
    }

    // Flush remaining table rows
    if !table_rows.is_empty() {
        lines.extend(render_table(&table_rows, bg));
    }

    // Close unclosed code block
    if in_code_block {
        push_code_block_close(&mut lines, bg);
    }

    lines
}
