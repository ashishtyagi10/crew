//! Table layout: column widths, padded cells, header rule. Split out of
//! `layout.rs` to keep that file under its line budget.
use crate::md::{ColAlign, LineKind, MdLine, MdSpan};

const SEP: &str = " │ ";

/// Spaces before and after a `cell_w`-wide cell in a `w`-wide column.
fn pads(align: ColAlign, w: usize, cell_w: usize) -> (usize, usize) {
    let gap = w.saturating_sub(cell_w);
    match align {
        ColAlign::Left => (0, gap),
        ColAlign::Right => (gap, 0),
        // The odd cell goes to the right, so a centred column's text starts
        // on the same side every row when the widths are odd.
        ColAlign::Center => (gap / 2, gap - gap / 2),
    }
}

/// Append `n` spaces, clipped to what is left of the `cols` budget.
fn push_pad(spans: &mut Vec<MdSpan>, acc: &mut usize, n: usize, cols: usize) {
    let n = n.min(cols.saturating_sub(*acc));
    if n > 0 {
        spans.push(super::wrap::plain_span(" ".repeat(n)));
        *acc += n;
    }
}

fn cell_text(spans: &[MdSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Display width of `s` (CJK/emoji count as 2 columns, combining marks as 0)
/// — table columns render on a fixed-width cell grid, so char-count padding
/// would misalign the `│` separator whenever a cell holds a wide glyph.
fn cell_width(spans: &[MdSpan]) -> usize {
    crate::chatwidth::str_w(&cell_text(spans))
}

fn col_widths(header: &[Vec<MdSpan>], rows: &[Vec<Vec<MdSpan>>]) -> Vec<usize> {
    let mut widths: Vec<usize> = header.iter().map(|c| cell_width(c)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let w = cell_width(cell);
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
fn row_line(
    cells: &[Vec<MdSpan>],
    widths: &[usize],
    aligns: &[ColAlign],
    bold: bool,
    cols: usize,
) -> MdLine {
    let mut spans = Vec::new();
    let mut acc = 0usize;
    let empty = Vec::new();
    for (i, &w) in widths.iter().enumerate() {
        if acc >= cols {
            break;
        }
        let cell = cells.get(i).unwrap_or(&empty);
        // Display width, not char count: a column's `w` (from `col_widths`)
        // is a display width too, so padding must close the same gap a
        // wide glyph (CJK/emoji) actually occupies on the cell grid, or the
        // `│` separator drifts out of alignment against other rows.
        let cell_w = cell_width(cell);
        let (lead, trail) = pads(aligns.get(i).copied().unwrap_or_default(), w, cell_w);
        push_pad(&mut spans, &mut acc, lead, cols);
        if acc < cols {
            for s in cell {
                let mut s = s.clone();
                if bold {
                    s.style.bold = true;
                }
                spans.push(s);
            }
            acc += cell_w;
        }
        push_pad(&mut spans, &mut acc, trail, cols);
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
    aligns: Vec<ColAlign>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    cols: usize,
) -> Vec<MdLine> {
    let widths = col_widths(&header, &rows);
    let mut out = vec![
        row_line(&header, &widths, &aligns, true, cols),
        rule_line(&widths, cols),
    ];
    out.extend(
        rows.iter()
            .map(|row| row_line(row, &widths, &aligns, false, cols)),
    );
    out
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
