//! The dimmed streaming tail: the last couple of rows of whatever an agent is
//! producing RIGHT NOW, drawn just above the live-run status line.
//!
//! It is an OVERFLOW view, not a second copy of the transcript. A growing
//! provisional card (`ChatPane::streaming`) is normally visible at the bottom
//! of the message area, and duplicating it there would be noise — so the tail
//! only appears when that card cannot be seen: the user has scrolled up, or
//! several agents are streaming at once so the newest is not the one drawing
//! the eye.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Rows the tail claims when it shows at all.
pub(crate) const TAIL_ROWS: u16 = 2;

/// The card the tail mirrors: the most recently updated one. `absorb_delta`
/// moves the card it touches to the end of `streaming`, so "last" is "newest"
/// without carrying a timestamp.
fn newest(pane: &ChatPane) -> Option<&Message> {
    pane.streaming.last()
}

/// What the tail WANTS: nothing unless a streaming card exists that the user
/// cannot already see. `grants` decides what it actually gets.
pub(crate) fn tail_rows(pane: &ChatPane, _cols: u16) -> u16 {
    let hidden = pane.scroll > 0 || pane.streaming.len() > 1;
    if hidden && newest(pane).is_some() {
        TAIL_ROWS
    } else {
        0
    }
}

/// Draw the tail into `TAIL_ROWS` rows starting at `start_row`: the last rows
/// of the newest card's text, wrapped to `cols` and muted.
pub(crate) fn tail_cells(pane: &ChatPane, cols: u16, start_row: u16) -> Vec<CellView> {
    let Some(card) = newest(pane) else {
        return Vec::new();
    };
    if cols == 0 {
        return Vec::new();
    }
    let theme = crew_theme::theme();
    let fg = theme.text_muted;
    let bg = theme.page_bg;
    // Reuse the transcript's own wrapper so the tail breaks text exactly the
    // way the card above it does.
    let chars: Vec<char> = card.text.chars().collect();
    let wrapped = crate::chatlayout::wrap_indices(&chars, cols as usize);
    let last: Vec<(usize, usize)> = wrapped
        .iter()
        .rev()
        .take(TAIL_ROWS as usize)
        .rev()
        .copied()
        .collect();
    let mut out = Vec::new();
    for (i, (s, e)) in last.iter().enumerate() {
        let row = start_row + i as u16;
        crate::chatwidth::place_row(
            0,
            cols,
            chars[*s..*e].iter().map(|&c| (c, fg)),
            |col, c, fg| {
                out.push(CellView {
                    col,
                    row,
                    c,
                    fg,
                    bg,
                    bold: false,
                    italic: false,
                });
            },
        );
    }
    out
}

#[cfg(test)]
#[path = "chattail_tests.rs"]
mod tests;
