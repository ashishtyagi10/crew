//! Table layout: column widths, padded cells, header rule. Split out of
//! `layout.rs` to keep that file under its line budget.
use crate::md::{LineKind, MdLine, MdSpan};

const SEP: &str = " │ ";

fn cell_text(spans: &[MdSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn col_widths(header: &[Vec<MdSpan>], rows: &[Vec<Vec<MdSpan>>]) -> Vec<usize> {
    let mut widths: Vec<usize> = header
        .iter()
        .map(|c| cell_text(c).chars().count())
        .collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let w = cell_text(cell).chars().count();
            if i < widths.len() {
                widths[i] = widths[i].max(w);
            } else {
                widths.push(w);
            }
        }
    }
    widths
}

fn total_width(widths: &[usize]) -> usize {
    if widths.is_empty() {
        0
    } else {
        widths.iter().sum::<usize>() + SEP.chars().count() * (widths.len() - 1)
    }
}

fn row_line(cells: &[Vec<MdSpan>], widths: &[usize], bold: bool, cols: usize) -> MdLine {
    let mut spans = Vec::new();
    for (i, &w) in widths.iter().enumerate() {
        let empty = Vec::new();
        let cell = cells.get(i).unwrap_or(&empty);
        let text_len = cell_text(cell).chars().count();
        for s in cell {
            let mut s = s.clone();
            if bold {
                s.style.bold = true;
            }
            spans.push(s);
        }
        if text_len < w {
            spans.push(super::wrap::plain_span(" ".repeat(w - text_len)));
        }
        if i + 1 < widths.len() {
            spans.push(super::wrap::plain_span(SEP.to_string()));
        }
    }
    MdLine {
        spans: super::wrap::truncate_spans(spans, cols),
        kind: LineKind::Body,
    }
}

fn rule_line(widths: &[usize], cols: usize) -> MdLine {
    let spans = vec![super::wrap::plain_span("─".repeat(total_width(widths)))];
    MdLine {
        spans: super::wrap::truncate_spans(spans, cols),
        kind: LineKind::Rule,
    }
}

/// Lays out a table: header line (bold), a `─` rule under it, then each data
/// row — all space-padded to each column's max cell width and hard-truncated
/// at `cols` if the table is wider than that.
pub(super) fn lines(
    header: Vec<Vec<MdSpan>>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    cols: usize,
) -> Vec<MdLine> {
    let widths = col_widths(&header, &rows);
    let mut out = vec![
        row_line(&header, &widths, true, cols),
        rule_line(&widths, cols),
    ];
    out.extend(rows.iter().map(|row| row_line(row, &widths, false, cols)));
    out
}
