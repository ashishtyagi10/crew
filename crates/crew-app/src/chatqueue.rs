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

/// Rows the queued-indicator claims in the message area: 0 when empty, else
/// exactly 1 (a single summary line, regardless of queue depth).
pub(crate) fn queued_rows(pane: &ChatPane) -> u16 {
    if pane.queued.is_empty() {
        0
    } else {
        1
    }
}

/// The indicator's text, or `None` when the queue is empty. Grammar
/// (message/messages) tracks the count.
pub(crate) fn indicator_text(pane: &ChatPane) -> Option<String> {
    let n = pane.queued.len();
    if n == 0 {
        return None;
    }
    let noun = if n == 1 { "message" } else { "messages" };
    Some(format!(
        "\u{29d7} {n} {noun} queued \u{2014} sends when the crew is idle"
    ))
}

/// Render the indicator at `row`, muted, starting one column in (matching
/// the swarm block's left inset).
pub(crate) fn indicator_cells(pane: &ChatPane, cols: u16, row: u16) -> Vec<CellView> {
    let Some(text) = indicator_text(pane) else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut cells = Vec::new();
    let mut col: u16 = 1;
    for c in text.chars() {
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
        });
        col += 1;
    }
    cells
}

#[cfg(test)]
#[path = "chatqueue_tests.rs"]
mod tests;
