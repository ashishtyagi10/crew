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

/// Builds one row's spans, then hard-truncates to `cols`. Column padding
/// pads out to each column's max width across the table, which can be huge
/// if one outlier row has a giant cell — so this stops accumulating spans
/// (and caps any padding string) as soon as `cols` is reached, rather than
/// materializing full-width padding for every column first and truncating
/// after. Keeps the cost bounded by `cols`, not by the widest cell.
fn row_line(cells: &[Vec<MdSpan>], widths: &[usize], bold: bool, cols: usize) -> MdLine {
    let mut spans = Vec::new();
    let mut acc = 0usize;
    let empty = Vec::new();
    for (i, &w) in widths.iter().enumerate() {
        if acc >= cols {
            break;
        }
        let cell = cells.get(i).unwrap_or(&empty);
        let text_len = cell_text(cell).chars().count();
        for s in cell {
            let mut s = s.clone();
            if bold {
                s.style.bold = true;
            }
            spans.push(s);
        }
        acc += text_len;
        if text_len < w {
            let pad = (w - text_len).min(cols.saturating_sub(acc));
            if pad > 0 {
                spans.push(super::wrap::plain_span(" ".repeat(pad)));
                acc += pad;
            }
        }
        if i + 1 < widths.len() {
            spans.push(super::wrap::plain_span(SEP.to_string()));
            acc += SEP.chars().count();
        }
    }
    MdLine {
        spans: super::wrap::truncate_spans(spans, cols),
        kind: LineKind::Body,
    }
}

fn rule_line(widths: &[usize], cols: usize) -> MdLine {
    // Never materialize more dashes than could ever be visible.
    let n = total_width(widths).min(cols);
    let spans = vec![super::wrap::plain_span("─".repeat(n))];
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

#[cfg(test)]
mod tests {
    use super::*;

    fn line_totals(lines: &[MdLine]) -> Vec<usize> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.text.chars().count()).sum())
            .collect()
    }

    /// One row has a huge cell in column 0; the rest have a one-char cell in
    /// that same column — so `col_widths` makes column 0's width huge even
    /// though almost every row's own content there is tiny.
    fn huge_cell_table(rows: usize, cell_len: usize) -> String {
        let mut s = String::from("| a | b |\n|---|---|\n");
        s.push_str(&format!("| {} | x |\n", "z".repeat(cell_len)));
        for _ in 1..rows {
            s.push_str("| 1 | x |\n");
        }
        s
    }

    #[test]
    fn one_huge_cell_does_not_blow_every_row_past_the_column_budget() {
        let s = huge_cell_table(200, 50_000);
        let lines = crate::md::render(&s, 80);
        for total in line_totals(&lines) {
            assert!(total <= 80, "line exceeds the {}-col budget: {total}", 80);
        }
    }

    #[test]
    fn table_layout_cost_is_bounded_by_the_column_budget_not_cell_size() {
        // Pre-fix, `row_line` pads every row out to the widest cell's FULL
        // width before truncating to `cols`, so one huge cell forces the
        // same huge padding allocation on every other (short) row, and
        // `rule_line` repeats `"─"` to the same huge total width. At this
        // input size (12,000 rows, one 3M-char cell) that measurably blows
        // past this 2s bound pre-fix (observed ~2.1s total, ~1.6s of it in
        // `table::lines` alone); post-fix `table::lines` drops to ~10ms,
        // since its cost is then bounded by `cols`, not cell size (the
        // remaining ~0.5s is unrelated md::parse cost of tokenizing one
        // huge cell, present before and after this fix).
        let s = huge_cell_table(12_000, 3_000_000);
        let start = std::time::Instant::now();
        let _ = crate::md::render(&s, 80);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "table layout took {elapsed:?} — cost scales with cell size, not the column budget"
        );
    }
}
