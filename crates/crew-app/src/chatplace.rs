//! Scroll-windowed card-line placement for the chat card view: shared by
//! `chatmsgs::message_cells` (drawing) and `clickopen`'s link hit-test (click
//! resolution) so both agree on exactly which line sits at which row.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::{CardCell, CardLine, Color};

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
    lines
        .into_iter()
        .skip(start)
        .take(end - start)
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

/// The `CardCell` occupying display column `col` on `line`, using the exact
/// same display-column accounting `line_cells` renders with (wide glyphs
/// advance `char_w` columns; zero-width marks are skipped and can never be
/// hit). Lets `chatview::link_at` map a click's display column back to its
/// cell without re-deriving `line_cells`' bookkeeping. `None` past the last
/// cell's column.
pub(crate) fn cell_at_col(line: &CardLine, col: u16) -> Option<&CardCell> {
    let mut acc: u16 = 0;
    for cell in line.iter() {
        let w = crate::chatwidth::char_w(cell.c) as u16;
        if w == 0 {
            continue; // zero-width marks are never hit targets
        }
        if col < acc + w {
            return Some(cell);
        }
        acc += w;
    }
    None
}

/// The message-area row budget for `pane`'s `cols` × `rows` grid: `rows`
/// minus the status rows above (session line + agent chips, via
/// `status_rows`), the composer rows below (via `composer_rows`), the live
/// swarm block (`chatswarmview::swarm_rows`), and the queued-messages
/// indicator (`chatqueue::queued_rows`) when either is showing. The single
/// source both `chatview::cells` and `placed_lines` call, so the two can
/// never drift apart on how many rows the message body gets.
pub(crate) fn msg_rows_budget(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    let top = pane.status_rows(cols, rows);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let block = crate::chatswarmview::swarm_rows(pane, rows);
    let queued = crate::chatqueue::queued_rows(pane);
    let prog = crate::chatprog::progress_rows(pane, cols);
    rows.saturating_sub(top + bottom + block + queued + prog)
}

/// The scroll-windowed card-line placement for `pane`'s message area, each
/// line tagged with its absolute row on the pane's `cols` × `rows` grid
/// (below `pane.status_rows`) — the same geometry `message_cells` draws.
/// `clickopen`'s link hit-test (`chatview::link_at`) reads this to map a
/// click back to its source line without re-deriving the card layout.
pub(crate) fn placed_lines(pane: &ChatPane, cols: u16, rows: u16) -> Vec<(u16, CardLine)> {
    if cols == 0 || rows == 0 || pane.messages.is_empty() {
        return Vec::new();
    }
    let top = pane.status_rows(cols, rows);
    if top == 0 {
        return Vec::new(); // too short for the card view (plain fallback)
    }
    let budget = msg_rows_budget(pane, cols, rows);
    if budget == 0 {
        return Vec::new();
    }
    let view = crate::chatmsgs::View {
        source: pane.show_source,
        compact: pane.compact_view,
    };
    let lines = crate::chatmsgs::card_lines(
        &pane.messages,
        cols as usize,
        crate::chattime::unix_now_ms(),
        view,
    );
    window(lines, budget, top, pane.scroll)
}
