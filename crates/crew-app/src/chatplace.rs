//! Scroll-windowed card-line placement for the chat card view: shared by
//! `chatmsgs::message_cells` (drawing) and Task 6's link hit-test (click
//! resolution) so both agree on exactly which line sits at which row.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::{CardLine, Color};

/// Scroll-window `lines` into `rows` rows, tagging each surviving line with
/// its absolute row (`top_row` + its offset in the window).
pub(crate) fn window(
    lines: Vec<CardLine>,
    rows: u16,
    top_row: u16,
    scroll: usize,
) -> Vec<(u16, CardLine)> {
    let max_start = lines.len().saturating_sub(rows as usize);
    let start = max_start.saturating_sub(scroll);
    let end = (start + rows as usize).min(lines.len());
    lines[start..end]
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, line)| (top_row + i as u16, line))
        .collect()
}

/// Map one already-placed `CardLine` to its `CellView`s at `row`, clipped to
/// `cols` (zero-width marks are dropped; wide glyphs advance two columns).
pub(crate) fn line_cells(row: u16, line: &CardLine, cols: u16, page: Color) -> Vec<CellView> {
    let mut cells = Vec::new();
    let mut col: u16 = 0;
    for cell in line.iter() {
        let w = crate::chatwidth::char_w(cell.c) as u16;
        if w == 0 {
            continue; // zero-width marks don't get their own cell
        }
        if col + w > cols {
            break;
        }
        cells.push(CellView {
            col,
            row,
            c: cell.c,
            fg: cell.fg,
            bg: cell.bg.unwrap_or(page),
            bold: cell.bold,
            italic: cell.italic,
        });
        col += w;
    }
    cells
}

/// The scroll-windowed card-line placement for `pane`'s message area, each
/// line tagged with its absolute row on the pane's `cols` × `rows_budget`
/// grid (below `pane.status_rows`) — the same geometry `message_cells` draws.
/// Task 6's link hit-test reads this to map a click back to its source line
/// without re-deriving the card layout.
#[allow(dead_code)] // consumed by Task 6's link hit-test; exercised by tests now
pub(crate) fn placed_lines(pane: &ChatPane, cols: u16, rows_budget: u16) -> Vec<(u16, CardLine)> {
    if cols == 0 || rows_budget == 0 || pane.messages.is_empty() {
        return Vec::new();
    }
    let top = pane.status_rows(cols, rows_budget);
    if top == 0 {
        return Vec::new(); // too short for the card view (plain fallback)
    }
    let bottom = crate::chatinput::composer_rows(rows_budget);
    let rows = rows_budget.saturating_sub(top + bottom);
    if rows == 0 {
        return Vec::new();
    }
    let lines = crate::chatmsgs::card_lines(
        &pane.messages,
        cols as usize,
        crate::chattime::unix_now_ms(),
    );
    window(lines, rows, top, pane.scroll)
}
