//! The dimmed streaming tail: the last couple of rows of whatever an agent is
//! producing RIGHT NOW, drawn just above the live-run status line.
//!
//! It is an OVERFLOW view, not a second copy of the transcript. A growing
//! provisional card (`ChatPane::streaming`) is normally visible at the bottom
//! of the message area, and duplicating it there would be noise. `chatplace::
//! window` is bottom-anchored: at `pane.scroll == 0` it always ends at the
//! newest line, for any row budget — shrinking `msg_rows` to make room for a
//! surface only moves the newest card up, it never pushes it off screen. So
//! the newest streaming card (last in `visible_messages()`, by construction)
//! is ALWAYS on screen at `scroll == 0`, no matter how many agents are
//! streaming at once — parallelism alone is not a reason to show the tail.
//! The tail therefore appears in exactly one case: the user has scrolled up
//! (`pane.scroll > 0`), so the live bottom — and the card growing there — is
//! genuinely off screen.
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

/// What the tail WANTS: nothing unless a streaming card exists AND the user
/// has scrolled away from the live bottom, where that card lives. `grants`
/// decides what it actually gets.
pub(crate) fn tail_rows(pane: &ChatPane, _cols: u16) -> u16 {
    if pane.scroll > 0 && newest(pane).is_some() {
        TAIL_ROWS
    } else {
        0
    }
}

/// Draw the tail into `TAIL_ROWS` rows starting at `start_row`: the last rows
/// of the newest card's rendered body, recoloured muted.
///
/// This renders through `chatbody::body_lines` — the exact function the
/// transcript itself calls for a message body (`chatmsgs::card_lines`) — with
/// the pane's own `show_source` flag, so the tail can never wrap, markdown-
/// style or word-break text differently than the card it mirrors. (An
/// earlier version re-wrapped the raw string with `chatlayout::wrap_indices`,
/// which is only the source-mode path; the default path renders through
/// `md::render_chat` + `chatmd::map_lines` instead, with different width
/// accounting and content — markdown would show raw in the tail while
/// rendered in the card above it.) Every cell's colour is then overwritten to
/// the muted tone, so the tail always reads as dim regardless of what the
/// card's markdown styled it as (headings, links, ...).
pub(crate) fn tail_cells(pane: &ChatPane, cols: u16, start_row: u16) -> Vec<CellView> {
    let Some(card) = newest(pane) else {
        return Vec::new();
    };
    if cols == 0 {
        return Vec::new();
    }
    let theme = crew_theme::theme();
    let muted = theme.text_muted;
    let page = theme.page_bg;
    let body = crate::chatbody::body_lines(&card.text, cols as usize, muted, pane.show_source);
    let last = body.iter().rev().take(TAIL_ROWS as usize).rev();
    let mut out = Vec::new();
    for (i, line) in last.enumerate() {
        let row = start_row + i as u16;
        let mut muted_line = line.clone();
        for cell in muted_line.iter_mut() {
            cell.fg = muted;
        }
        out.extend(crate::chatplace::line_cells(row, &muted_line, cols, page));
    }
    out
}

#[cfg(test)]
#[path = "chattail_tests.rs"]
mod tests;
