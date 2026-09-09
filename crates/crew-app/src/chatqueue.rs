//! Messages typed while the crew is busy: queued instead of sent immediately
//! (see the queued-messages design doc), then flushed one at a time as each
//! turn settles (the flush itself lives in `chat::ChatPane::poll`, since it
//! needs private field access). This module holds the pure bits: the `/stop`
//! bypass check and the one-line "N queued" indicator that claims a row
//! above the composer, mirroring how `chatswarmview::swarm_rows` claims rows
//! for the live swarm block.
use crew_render::CellView;

use crate::chat::ChatPane;

/// Whether `text` is (or starts) a `/stop` command — the one send that must
/// bypass the queue and reach the broker immediately even while busy, since
/// it's the cancel path for the in-flight run.
pub(crate) fn is_stop(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "/stop" || trimmed.starts_with("/stop ")
}

/// Whether `text` cancels EVERYTHING, i.e. a bare `/stop` with no task id.
///
/// The distinction matters because cancelling is what makes the pane idle,
/// and idle is what flushes the queue: a cancel-everything must take the
/// queue with it or it restarts the moment it lands. `/stop #2` is the other
/// thing — one of several parallel tasks called off, with the rest of the
/// session, and the rest of the queue, still meant.
pub(crate) fn is_stop_all(text: &str) -> bool {
    text.trim() == "/stop"
}

/// Rows the queued-indicator claims in the message area: 0 when empty, else
/// exactly 1 (a single summary line, regardless of queue depth).
pub(crate) fn queued_rows(pane: &ChatPane) -> u16 {
    if pane.queued.is_empty() {
        0
    } else {
        1
    }
}

/// One turn of the queued hourglass, at full motion (Subtle: 1.6× slower).
pub(crate) const HOURGLASS_MS: u64 = 600;

/// The indicator's hourglass at `now_ms`: the two glyphs of the pair
/// (`glyphs::Glyph::Hourglass`, `⧗ ⧖` on a plain font) alternate every
/// [`HOURGLASS_MS`] while anything is queued — sand still running — and Off
/// holds the first, still.
pub(crate) fn hourglass(now_ms: u64, level: crate::motion::MotionLevel) -> char {
    let period = crate::shimmer::period(HOURGLASS_MS, level);
    let frame = now_ms
        .checked_div(period)
        .map_or(0, |turns| (turns % 2) as u8);
    crate::glyphs::pick_char(crate::glyphs::Glyph::Hourglass(frame))
}

/// The indicator's text at `now_ms`, or `None` when the queue is empty.
/// Grammar (message/messages) tracks the count.
pub(crate) fn indicator_text(pane: &ChatPane, now_ms: u64) -> Option<String> {
    let n = pane.queued.len();
    if n == 0 {
        return None;
    }
    let noun = if n == 1 { "message" } else { "messages" };
    let glass = hourglass(now_ms, crate::motion::level());
    Some(format!(
        "{glass} {n} {noun} queued \u{2014} sends when the crew is idle"
    ))
}

/// Render the indicator at `row` on the animation clock — see
/// [`indicator_cells_at`].
pub(crate) fn indicator_cells(pane: &ChatPane, cols: u16, row: u16) -> Vec<CellView> {
    indicator_cells_at(pane, cols, row, crate::anim::now_ms())
}

/// Render the indicator at `row`, muted, starting one column in (matching
/// the swarm block's left inset); `now_ms` turns the hourglass.
pub(crate) fn indicator_cells_at(
    pane: &ChatPane,
    cols: u16,
    row: u16,
    now_ms: u64,
) -> Vec<CellView> {
    let Some(text) = indicator_text(pane, now_ms) else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut cells = Vec::new();
    for (col, c) in (1u16..).zip(text.chars()) {
        if col >= cols {
            break;
        }
        cells.push(CellView {
            col,
            row,
            c,
            fg: theme.text_muted,
            bg: theme.page_bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    }
    cells
}

#[cfg(test)]
#[path = "chatqueue_tests.rs"]
mod tests;
