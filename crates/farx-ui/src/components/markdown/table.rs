//! Markdown table rendering with box-drawing borders.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render a markdown table with box-drawing borders.
pub(super) fn render_table(rows: &[&str], bg: Color) -> Vec<Line<'static>> {
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
