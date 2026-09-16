//! Drawing the queued indicator: the summary line, the list under it, and
//! the hourglass that says the sand is still running.
//!
//! Split from [`crate::chatqueue`] when the indicator stopped being one row.
//! That file answers "what is waiting and how much room does it need"; this
//! one answers "what does it look like", and the two were only ever together
//! because the answer used to be short enough not to matter.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatqueue::listed;

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
pub(crate) fn indicator_cells(pane: &ChatPane, cols: u16, row: u16, rows: u16) -> Vec<CellView> {
    indicator_cells_at(pane, cols, row, rows, crate::anim::now_ms())
}

/// Render the indicator from `row` down, muted, starting one column in
/// (matching the swarm block's left inset); `now_ms` turns the hourglass.
///
/// `rows` is what the layout GRANTED, not what the queue wants: the summary
/// is drawn first and the list fills whatever is left, so a pane with one row
/// to spare still says how many are waiting.
pub(crate) fn indicator_cells_at(
    pane: &ChatPane,
    cols: u16,
    row: u16,
    rows: u16,
    now_ms: u64,
) -> Vec<CellView> {
    let Some(head) = indicator_text(pane, now_ms) else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut cells = Vec::new();
    let mut lines = vec![head];
    lines.extend(listed(pane));
    for (i, text) in lines.into_iter().take(usize::from(rows)).enumerate() {
        // The list is indented under the line that counts it, so the two read
        // as a heading and its contents rather than five equal notices.
        let inset = if i == 0 { 1 } else { 3 };
        // Marked and width-aware, as every other notice on the canvas: a
        // half-width tile used to read `…sends when the c` with no cut mark.
        let text = crate::chatwidth::clip_w(&text, usize::from(cols.saturating_sub(inset)));
        let styled = text.chars().map(|c| (c, theme.text_muted));
        let row = row + i as u16;
        crate::chatwidth::place_row(inset, cols, styled, |col, c, fg| {
            cells.push(CellView {
                col,
                row,
                c,
                fg,
                bg: theme.page_bg,
                ..Default::default()
            })
        });
    }
    cells
}

#[cfg(test)]
#[path = "chatqueuedraw_tests.rs"]
mod tests;
