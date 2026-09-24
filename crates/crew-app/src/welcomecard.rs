//! The welcome's card: the one card on an empty canvas, drawn as the lone
//! focused terminal it stands in for.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;

/// [`crate::panelcard::push_card`] wearing the FOCUSED ring instead of the quiet stroke — for
/// the welcome, the one card on an empty canvas. It occupies exactly the rect
/// a lone Cmd+T terminal would, and that terminal is drawn lit; drawn in
/// `border_normal` instead, the modern light pages (a ~2:1 stroke, whitened
/// further by the quiet gradient) left the main page with no visible edge at
/// all.
pub fn push_card_lit(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    cw: f32,
    ch: f32,
    legend: &str,
    content: impl FnOnce(u16, u16) -> Vec<CellView>,
) {
    let t = crew_theme::theme();
    let (icols, irows) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    let (cols, rows) = (icols + 2, irows + 2);
    let mut frame = crate::boxdraw::titled_card(
        cols,
        rows,
        legend,
        crate::panecardglow::focused_stroke(t),
        t.ink,
        t.page_bg,
    );
    crate::modernring::ring(&mut frame, cols, rows, false, 1.0, 0);
    crate::panelcard::push_framed(
        scenes,
        rect,
        cw,
        ch,
        frame,
        (content(icols, irows), Vec::new()),
        // Lit is the whole of it: the stroke AND the elevation a focused
        // terminal's glass rides at (`spotlight::lift_for` at rest). At 0
        // the sheet lay flat on the page — a lit line round a card that
        // hadn't risen, and on a sheer window not even the line read.
        1.0,
    );
}

#[cfg(test)]
#[path = "welcomecard_tests.rs"]
mod tests;
