//! Table layout: column widths, padded cells, header rule. Split out of
//! `layout.rs` to keep that file under its line budget.
use crate::md::{LineKind, MdLine, MdSpan};

const SEP: &str = " │ ";

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
fn row_line(cells: &[Vec<MdSpan>], widths: &[usize], bold: bool, cols: usize) -> MdLine {
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
        for s in cell {
            let mut s = s.clone();
            if bold {
                s.style.bold = true;
            }
            spans.push(s);
        }
        acc += cell_w;
        if cell_w < w {
            let pad = (w - cell_w).min(cols.saturating_sub(acc));
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
    fn table_aligns_wide_glyph_cells() {
        // 漢字 is two 2-column-wide glyphs (4 display columns, 2 chars) —
        // char-count padding would treat it as width 2, misaligning the
        // separator against the header row's char-count-2 "b" column... but
        // the real bug is column 0: "漢字" (2 chars, 4 display cols) vs "a"
        // (1 char, 1 display col) — char-count padding overshoots by the
        // wide cell's extra display width.
        let lines = crate::md::render("| a | b |\n|---|---|\n| \u{6f22}\u{5b57} | x |", 40);
        fn prefix_display_width(line: &MdLine) -> usize {
            let text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
            let prefix = text.split('\u{2502}').next().unwrap_or("");
            crate::chatwidth::str_w(prefix)
        }
        let header_w = prefix_display_width(&lines[0]);
        let data_w = prefix_display_width(&lines[2]);
        assert_eq!(
            header_w, data_w,
            "the │ separator should land at the same display column: header={header_w} data={data_w}"
        );
    }

    /// The clamp and the padding must speak the same unit. `row_line`
    /// accumulates DISPLAY width against `cols`, so the final truncate has
    /// to clamp display columns too — with char-count truncation a CJK
    /// table keeps `cols` CHARS (up to 2x the display budget) and the
    /// over-wide row wraps or spills in the pane.
    #[test]
    fn wide_glyph_rows_respect_the_display_column_budget() {
        let lines = crate::md::render(
            "| \u{6f22}\u{5b57}\u{6f22}\u{5b57} | \u{6f22}\u{5b57} |\n|---|---|\n| \u{6f22}\u{5b57}\u{6f22}\u{5b57} | \u{6f22}\u{5b57} |",
            10,
        );
        for (i, l) in lines.iter().enumerate() {
            let text: String = l.spans.iter().map(|s| s.text.as_str()).collect();
            let w = crate::chatwidth::str_w(&text);
            assert!(
                w <= 10,
                "line {i} is {w} display cols wide (budget 10): {text:?}"
            );
        }
        // And a boundary-straddling wide glyph is dropped, never split or
        // kept: "ab<CJK>" clamped to 3 display cols keeps exactly "ab".
        let spans = vec![super::super::wrap::plain_span("ab\u{6f22}".to_string())];
        let out = super::super::wrap::truncate_spans(spans, 3);
        let text: String = out.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "ab");
    }

    /// Pre-fix, `row_line` padded every row out to the widest cell's FULL
    /// width before truncating to `cols`, so one huge cell forced the same
    /// huge padding allocation on every other (short) row, and `rule_line`
    /// repeated `"─"` to that same huge total width. At this size (12,000
    /// rows, one 3M-char cell) `lines` takes ~10ms with the fix and 452s
    /// without it — measured by reverting the two `.min(cols)` clamps below
    /// and re-running this test — since its cost is now bounded by `cols`
    /// rather than by cell size.
    ///
    /// The measurement is a **ratio, not a stopwatch**. Two earlier versions
    /// of this test asserted an absolute bound and both failed on Windows CI
    /// without anything having regressed — first the whole render against 2s
    /// (2.65s observed), then `lines` alone against 0.5s (512ms observed).
    /// A shared runner is simply slower than the machine the bound was picked
    /// on, and picking a bigger number only moves the next false failure.
    ///
    /// So: lay out the same table twice, identical in every way except the
    /// width of one cell. If cost scaled with cell size the wide run would be
    /// thousands of times the narrow one; with the clamps it is ~1x. Anything
    /// under a 50x factor proves the invariant with room to spare, and a slow
    /// machine slows *both* runs, so the ratio holds.
    ///
    /// The sizes are chosen so the *unclamped* version fails in seconds rather
    /// than hanging. At 3,000,000 chars x 12,000 rows a regression ran past
    /// three minutes without finishing, which in CI reads as a stuck job
    /// instead of a failed assertion.
    #[test]
    fn table_layout_cost_is_bounded_by_the_column_budget_not_cell_size() {
        let cell = |t: &str| vec![super::super::wrap::plain_span(t.to_string())];
        // Same shape both times; only the first cell's width differs.
        let table = |width: usize| {
            let header = vec![cell("a"), cell("b")];
            let mut rows = vec![vec![cell(&"z".repeat(width)), cell("x")]];
            rows.extend((1..1_200).map(|_| vec![cell("1"), cell("x")]));
            (header, rows)
        };
        let time_it = |width: usize| {
            let (header, rows) = table(width);
            let start = std::time::Instant::now();
            let out = lines(header, rows, 80);
            (start.elapsed(), out)
        };

        // Narrow first, so any one-off warm-up is charged to the baseline
        // rather than to the run under scrutiny.
        let (narrow, _) = time_it(80);
        let (wide, out) = time_it(300_000);

        // The invariant itself, not just its timing: nothing wider than the
        // budget was ever handed back, however wide the widest cell was.
        for total in line_totals(&out) {
            assert!(total <= 80, "line exceeds the 80-col budget: {total}");
        }

        // A floor on the baseline keeps the ratio meaningful when the narrow
        // run is too fast to time; without it a sub-microsecond baseline makes
        // any wide time look like a huge multiple.
        let baseline = narrow.as_secs_f64().max(0.005);
        let ratio = wide.as_secs_f64() / baseline;
        assert!(
            ratio < 50.0,
            "laying out a 300,000-char cell took {ratio:.0}x as long as an \
             80-char one ({wide:?} vs {narrow:?}) — cost is scaling with cell \
             size again, not the column budget"
        );
    }
}
