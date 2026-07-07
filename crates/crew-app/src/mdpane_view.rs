//! `MdPane::cells`: draws the numbered source half, the muted divider
//! column, and the styled preview half — both halves' lines come from
//! `MdPane::cache_for` (`mdcache`), shared with `MdPane::link_at` — into
//! one `CellView` list.
use crew_render::CellView;

use crate::chatbody::CardLine;
use crate::mdcache::GUTTER_W;
use crate::mdpane::{geometry, window_top, MdPane, Side};

impl MdPane {
    /// Caps both scroll offsets to their content's last full page at this
    /// `cols`×`rows` geometry. `window_top` already clamps the rendered
    /// *view* to a valid window, but the stored offset itself was
    /// unbounded — a huge jump (Shift+End's `i32::MIN/2` delta) left it
    /// sitting around content length forever, so every later Up/wheel-up
    /// tick just decremented that huge number with no visible motion.
    /// `cols == 0 || rows == 0` means no geometry yet (mirrors `cells`'s
    /// zero-size guard) — offsets are left alone.
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

    /// Renders both halves of the split into `cols` × `rows` cells. Never
    /// panics: `cols == 0 || rows == 0` (and any `cols` too small for a
    /// half) simply draw less, down to just the divider column. Reads
    /// wrapped-source/preview lines through `cache_for` rather than
    /// re-parsing the whole file every redraw.
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

/// The source half: a 4-col right-aligned muted line number + space, then
/// the hard-wrapped raw text (precomputed by `mdcache::wrap_source`,
/// reached via `MdPane::cache_for`), scrolled top-anchored by `scroll`.
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
/// chat (`mdpane::preview_lines`, precomputed by `MdPane::cache_for`),
/// scrolled top-anchored by `scroll` and shifted `right_start` columns right
/// of the divider.
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
