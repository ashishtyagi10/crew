//! `MdPane::cells`: draws the numbered source half, the muted divider, and
//! the styled preview half — lines come from `MdPane::cache_for`
//! (`mdcache`), shared with `MdPane::link_at`.
use crew_render::CellView;

use crate::chatbody::CardLine;
use crate::mdcache::GUTTER_W;
use crate::mdpane::{geometry, window_top, MdPane, Side};

impl MdPane {
    /// Caps both scroll offsets to their content's last full page for this
    /// `cols`×`rows` geometry — `window_top` clamps the rendered *view*, but
    /// not the stored offset, which a huge jump (Shift+End) could otherwise
    /// leave stuck, deadening every later Up/wheel-up tick. `0` means no
    /// geometry yet (mirrors `cells`'s zero-size guard) — offsets untouched.
    pub(crate) fn clamp_scrolls(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        let (left_w, _, _, right_w) = geometry(cols);
        let (src_len, prev_len) = {
            let cache = self.cache_for(cols);
            (cache.wrapped_src.len(), cache.preview.len())
        };
        if left_w > 0 {
            self.scroll_src = self.scroll_src.min(src_len.saturating_sub(rows as usize));
        }
        if right_w > 0 {
            self.scroll_prev = self.scroll_prev.min(prev_len.saturating_sub(rows as usize));
        }
    }

    /// Renders both halves into `cols` × `rows` cells via `cache_for`. Never
    /// panics: `cols == 0 || rows == 0` (or any `cols` too small for a half)
    /// simply draws less, down to just the divider column.
    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        let (left_w, divider_col, right_start, right_w) = geometry(cols);
        let mut out = divider_cells(divider_col, rows);
        {
            let cache = self.cache_for(cols);
            if left_w > 0 {
                out.extend(source_cells(
                    &cache.wrapped_src,
                    left_w,
                    rows,
                    self.scroll_src,
                ));
            }
            if right_w > 0 {
                out.extend(preview_cells(
                    &cache.preview,
                    right_w,
                    rows,
                    self.scroll_prev,
                    right_start,
                ));
            }
        }
        let has_room = match self.active {
            Side::Source => left_w > 0,
            Side::Preview => right_w > 0,
        };
        // See `indicator_overdraws`: a 1000+ scrolled-to source top line
        // shouldn't have its gutter digit overwritten.
        if has_room {
            let indicator = indicator_cell(self.active, right_start);
            if !indicator_overdraws(&out, &indicator) {
                out.retain(|c| !(c.row == indicator.row && c.col == indicator.col));
                out.push(indicator);
            }
        }
        out
    }
}

/// Would drawing `indicator` overwrite a non-blank cell? True only when a
/// scrolled-to source top line hits 1000+ (its gutter loses its left
/// padding) — the caller skips the indicator there rather than corrupt the
/// digit; it just won't show at that one scroll position on huge files.
fn indicator_overdraws(out: &[CellView], indicator: &CellView) -> bool {
    out.iter()
        .find(|c| c.row == indicator.row && c.col == indicator.col)
        .is_some_and(|c| c.c != ' ')
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

/// The source half: a 4-col right-aligned muted line number + space, then
/// the hard-wrapped text from `mdcache::wrap_source` (via `cache_for`).
/// Columns advance by display width (`chatwidth::char_w`), like
/// `chatplace::line_cells`, so a wide CJK/emoji glyph can't overlap what
/// follows it.
fn source_cells(
    wrapped: &[(usize, Vec<char>)],
    left_w: usize,
    rows: u16,
    scroll: usize,
) -> Vec<CellView> {
    let (start, end) = window_top(wrapped.len(), rows as usize, scroll);
    let muted = crew_theme::theme().text_muted;
    let ink = crew_theme::theme().ink;
    let page = crew_theme::theme().page_bg;
    let gutter_w = GUTTER_W.min(left_w) as u16;
    let left_w = left_w as u16;
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
        let row = row_i as u16;
        let mut col: u16 = 0;
        for &c in gutter.iter().chain(chunk.iter()) {
            let w = crate::chatwidth::char_w(c) as u16;
            if w == 0 {
                continue; // zero-width marks don't get their own cell
            }
            if col + w > left_w {
                break; // stops at the half's budget, same as `chatplace::line_cells`
            }
            let fg = if col < gutter_w { muted } else { ink };
            out.push(CellView {
                col,
                row,
                c,
                fg,
                bg: page,
                bold: false,
                italic: false,
            });
            col += w;
        }
    }
    out
}

/// The preview half: precomputed card lines (`cache_for`) placed like chat
/// (`chatplace::line_cells`), shifted `right_start` columns past the divider.
fn preview_cells(
    lines: &[CardLine],
    right_w: usize,
    rows: u16,
    scroll: usize,
    right_start: u16,
) -> Vec<CellView> {
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
