//! How a section divides the panel's rows — the one [`NavLayout`] the draw, the paint layer and
//! every hit path read for the frame, now per section (see [`crate::navsection`]).
//!
//! A section draws only its own rows; everything it does not draw is laid out as absent — no
//! LOG lines, no WAITING rows, no pane list — so a hit path asking "is this row the pane list?"
//! gets "there is none" rather than a row somewhere off the panel.
use crate::navlayout::{layout_from, NavLayout, Tail, LOG_MIN};
use crate::navsection::NavSection;

/// The panel's division for `section`, at `rows` content rows. `tail` is the slot's filling
/// (`/nav`: the glance cards or the LOG alone); `log_len` the LOG's entry count, which Activity
/// shows under the cards either way.
pub fn layout_for(
    section: NavSection,
    rows: u16,
    tail: Tail,
    log_len: usize,
    panes: usize,
) -> NavLayout {
    match section {
        // The machine's stack is fixed rows, drawn from the top (`statspane::system_cells`).
        NavSection::System | NavSection::Explorer | NavSection::Inbox | NavSection::Far => {
            NavLayout::default()
        }
        NavSection::Panes => NavLayout {
            panes_top: (panes > 0).then_some(0),
            ..Default::default()
        },
        NavSection::Activity => {
            let (mut out, end) = layout_from(0, rows, tail, 0);
            // Under the cards, the LOG takes the rest of the column — it is the one thing
            // here with more to say than it has room for. With `/nav log` there are no cards,
            // and the slot above already IS the LOG, grown to the column.
            if matches!(tail, Tail::Glance { .. }) {
                let room = rows.saturating_sub(end).saturating_sub(2) as usize;
                let lines = log_len.min(room);
                out.log_top = end;
                out.log_lines = if lines < LOG_MIN { 0 } else { lines };
            } else {
                let room = rows.saturating_sub(2) as usize;
                let lines = log_len.min(room);
                out.log_lines = if lines < LOG_MIN { 0 } else { lines };
            }
            out
        }
    }
}

#[cfg(test)]
#[path = "navsectionlayout_tests.rs"]
mod tests;
