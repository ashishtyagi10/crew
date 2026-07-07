//! `MdPane::cells`: draws the numbered source half, the muted divider
//! column, and the styled preview half (via `mdpane::preview_lines`, shared
//! with `MdPane::link_at`) into one `CellView` list.
use crew_render::CellView;

use crate::mdpane::{geometry, preview_lines, window_top, MdPane, Side};

/// Right-aligned line-number width (4 digits) plus one separating space.
const GUTTER_W: usize = 5;

impl MdPane {
    /// Renders both halves of the split into `cols` × `rows` cells. Never
    /// panics: `cols == 0 || rows == 0` (and any `cols` too small for a
    /// half) simply draw less, down to just the divider column.
    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        let (left_w, divider_col, right_start, right_w) = geometry(cols);
        let mut out = divider_cells(divider_col, rows);
        if left_w > 0 {
            out.extend(source_cells(&self.source, left_w, rows, self.scroll_src));
        }
        if right_w > 0 {
            out.extend(preview_cells(
                &self.source,
                right_w,
                rows,
                self.scroll_prev,
                right_start,
            ));
        }
        let has_room = match self.active {
            Side::Source => left_w > 0,
            Side::Preview => right_w > 0,
        };
        if has_room {
            let indicator = indicator_cell(self.active, right_start);
            out.retain(|c| !(c.row == indicator.row && c.col == indicator.col));
            out.push(indicator);
        }
        out
    }
}

/// One-cell `▸` marking which half of the split has focus, drawn over
/// whatever else sits at that pane-relative cell — top row, column 0 for the
/// source half or `right_start` for the preview half.
fn indicator_cell(active: Side, right_start: u16) -> CellView {
    let col = match active {
        Side::Source => 0,
        Side::Preview => right_start,
    };
    CellView {
        col,
        row: 0,
        c: '\u{25B8}', // ▸
        fg: crew_theme::theme().ink,
        bg: crew_theme::theme().page_bg,
        bold: false,
        italic: false,
    }
}

/// The muted `│` divider, drawn the full height regardless of content — the
/// active side is marked by `indicator_cell`'s `▸`, not by this divider.
fn divider_cells(divider_col: u16, rows: u16) -> Vec<CellView> {
    let muted = crew_theme::theme().text_muted;
    let page = crew_theme::theme().page_bg;
    (0..rows)
        .map(|row| CellView {
            col: divider_col,
            row,
            c: '\u{2502}',
            fg: muted,
            bg: page,
            bold: false,
            italic: false,
        })
        .collect()
}

/// Hard-wraps `source` at `text_w` display columns, tagging every wrapped
/// row with its 1-based source line number (continuation rows share their
/// line's number so callers know whether to reprint the gutter digits).
fn wrap_source(source: &str, text_w: usize) -> Vec<(usize, Vec<char>)> {
    let mut out = Vec::new();
    for (i, line) in source.split('\n').enumerate() {
        let n = i + 1;
        let chars: Vec<char> = line.chars().collect();
        if text_w == 0 || chars.is_empty() {
            out.push((n, Vec::new()));
            continue;
        }
        let mut s = 0;
        while s < chars.len() {
            let e = crate::chatwidth::fit_end(&chars, s, text_w);
            out.push((n, chars[s..e].to_vec()));
            s = e;
        }
    }
    out
}

/// The source half: a 4-col right-aligned muted line number + space, then
/// the hard-wrapped raw text, scrolled top-anchored by `scroll`.
fn source_cells(source: &str, left_w: usize, rows: u16, scroll: usize) -> Vec<CellView> {
    let text_w = left_w.saturating_sub(GUTTER_W);
    let wrapped = wrap_source(source, text_w);
    let (start, end) = window_top(wrapped.len(), rows as usize, scroll);
    let muted = crew_theme::theme().text_muted;
    let ink = crew_theme::theme().ink;
    let page = crew_theme::theme().page_bg;
    let gutter_w = GUTTER_W.min(left_w);
    let mut out = Vec::new();
    let mut prev_line_no = None;
    for (row_i, idx) in (start..end).enumerate() {
        let (line_no, chunk) = &wrapped[idx];
        let is_first_row_of_line = prev_line_no != Some(*line_no);
        prev_line_no = Some(*line_no);
        let gutter: Vec<char> = if is_first_row_of_line {
            format!("{line_no:>4} ").chars().collect()
        } else {
            vec![' '; GUTTER_W]
        };
        let mut row_chars = gutter;
        row_chars.extend(chunk.iter().copied());
        row_chars.truncate(left_w);
        let row = row_i as u16;
        for (col_i, &c) in row_chars.iter().enumerate() {
            let fg = if col_i < gutter_w { muted } else { ink };
            out.push(CellView {
                col: col_i as u16,
                row,
                c,
                fg,
                bg: page,
                bold: false,
                italic: false,
            });
        }
    }
    out
}

/// The preview half: `md::render` mapped to styled card lines exactly like
/// chat (`mdpane::preview_lines`), scrolled top-anchored by `scroll` and
/// shifted `right_start` columns right of the divider.
fn preview_cells(
    source: &str,
    right_w: usize,
    rows: u16,
    scroll: usize,
    right_start: u16,
) -> Vec<CellView> {
    let lines = preview_lines(source, right_w);
    let (start, end) = window_top(lines.len(), rows as usize, scroll);
    let page = crew_theme::theme().page_bg;
    let mut out = Vec::new();
    for (row_i, idx) in (start..end).enumerate() {
        let row = row_i as u16;
        for mut cell in crate::chatplace::line_cells(row, &lines[idx], right_w as u16, page) {
            cell.col += right_start;
            out.push(cell);
        }
    }
    out
}
