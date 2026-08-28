//! Scroll-windowed card-line placement for the chat card view: shared by
//! `chatmsgs::message_cells` (drawing) and `clickopen`'s link hit-test (click
//! resolution) so both agree on exactly which line sits at which row.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::{CardCell, CardLine, Color};

/// Scroll-window `lines` into `rows` rows, tagging each surviving line with
/// its absolute row.
///
/// A transcript shorter than its window sits on the BOTTOM of it, against the
/// composer, and the slack goes above — the way a shell's output sits above
/// its prompt and the way every chat reads. Anchoring it to the top instead
/// left a session's first few turns floating with eight blank rows between
/// the newest card and the box you answer it in, so the two things that
/// belong together were the two furthest apart on screen.
pub(crate) fn window(
    lines: Vec<CardLine>,
    rows: u16,
    top_row: u16,
    scroll: usize,
) -> Vec<(u16, CardLine)> {
    let max_start = lines.len().saturating_sub(rows as usize);
    let start = max_start.saturating_sub(scroll);
    let end = (start + rows as usize).min(lines.len());
    let pad = top_pad(lines.len(), rows);
    lines
        .into_iter()
        .skip(start)
        .take(end - start)
        .enumerate()
        .map(|(i, line)| (top_row + pad + i as u16, line))
        .collect()
}

/// Rows of slack above a transcript that does not fill its window — where
/// [`window`] puts its first line. Only ever positive when the whole
/// transcript fits, so a scrolled-back or overflowing view is placed exactly
/// where it always was.
///
/// ONE definition, read by the draw and by every path that resolves a click
/// back to a line (`chatfold::line_index_at`). Two row divisions is how the
/// left nav shipped a hit-test one row off its own frame.
pub(crate) fn top_pad(total: usize, rows: u16) -> u16 {
    (rows as usize).saturating_sub(total) as u16
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
            ..Default::default()
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

/// How a pane's rows are allotted. Decided in ONE place so the rows a surface
/// is budgeted and the rows it draws can never disagree.
///
/// Each `*_rows` function says what its surface WANTS; this says what it gets.
/// A pane too short for all of them drops the ones that don't fit rather than
/// piling them: the anchors used to floor at `.max(top)`, which collapsed the
/// bar, the indicator and the status line onto the same row and let
/// last-write-wins hide whichever drew first.
///
/// Priority, most expendable last: the status line, then the queued indicator,
/// then the bar. The bar goes first because the status line now carries the
/// `done/total` counter that used to be the bar's label — a pane with room for
/// one of them is better off with the one that also names the task.
pub(crate) struct Grants {
    pub top: u16,
    pub bottom: u16,
    /// The whole-pane summary footer below the composer (0 or 1 row).
    pub summary: u16,
    pub swarm: u16,
    pub queued: u16,
    pub prog: u16,
    /// The dimmed streaming overflow tail (0 or `TAIL_ROWS`).
    pub tail: u16,
    /// What's left for the transcript.
    pub msg: u16,
}

pub(crate) fn grants(pane: &ChatPane, cols: u16, rows: u16) -> Grants {
    let top = pane.status_rows(cols, rows);
    // The summary footer claims the very bottom row; the composer sits in what
    // remains, so its height is measured against `rows - summary`.
    let summary = crate::chatsummary::summary_rows(pane, cols, rows);
    let composer = crate::chatinput::composer_rows(&pane.input, cols, rows - summary);
    let bottom = composer + summary;
    // The header and the composer (+ its summary footer) are load-bearing
    // chrome and are never dropped; everything else shares what's between them.
    let mut left = rows.saturating_sub(top).saturating_sub(bottom);
    let mut take = |want: u16| {
        let got = want.min(left);
        left -= got;
        got
    };
    let swarm = take(crate::chatswarmview::swarm_rows(pane, cols));
    let queued = take(crate::chatqueue::queued_rows(pane));
    let prog = take(crate::chatprog::progress_rows(pane, cols));
    // The tail is the most expendable surface: the text it mirrors also
    // exists in the transcript (or the streaming card itself), so it takes
    // last, after everything load-bearing has already claimed its rows.
    let tail = take(crate::chattail::tail_rows(pane, cols));
    Grants {
        top,
        bottom,
        summary,
        swarm,
        queued,
        prog,
        tail,
        msg: left,
    }
}

/// The message-area row budget for `pane`'s `cols` × `rows` grid: what's left
/// after the status rows above, the composer below, and whichever live-run
/// surfaces [`grants`] could seat. The single source both `chatview::cells`
/// and `placed_lines` call, so the two can never drift apart on how many rows
/// the message body gets.
pub(crate) fn msg_rows_budget(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    grants(pane, cols, rows).msg
}

/// The scroll-windowed card-line placement for `pane`'s message area, each
/// line tagged with its absolute row on the pane's `cols` × `rows` grid
/// (below `pane.status_rows`) — the same geometry `message_cells` draws.
/// `clickopen`'s link hit-test (`chatview::link_at`) reads this to map a
/// click back to its source line without re-deriving the card layout.
pub(crate) fn placed_lines(pane: &ChatPane, cols: u16, rows: u16) -> Vec<(u16, CardLine)> {
    let visible = pane.visible_messages();
    if cols == 0 || rows == 0 || visible.is_empty() {
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
        gap_rows: crate::density::level().card_gap_rows(),
        source: pane.show_source,
        compact: pane.compact_view,
        // `visible_messages` chains settled messages then streaming ones, so
        // everything from that boundary on is still arriving.
        streaming_from: pane.messages.len(),
    };
    let lines = crate::chatmsgs::card_lines(
        &visible,
        cols as usize,
        crate::chattime::unix_now_ms(),
        view,
    );
    window(lines, budget, top, pane.scroll)
}

#[cfg(test)]
#[path = "chatplace_tests.rs"]
mod tests;
