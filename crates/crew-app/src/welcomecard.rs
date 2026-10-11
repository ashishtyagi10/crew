//! The welcome's cards: the one card on an empty canvas, drawn as the lone
//! focused terminal it stands in for, and — risen off it — the glass the
//! rain, the name and the hints sit on.
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

/// The whole welcome: the lit card in `rect`, and the rain block raised on
/// its own glass inside it (see [`push_block`]).
pub fn push_welcome(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    cw: f32,
    ch: f32,
    tick: u64,
    restore: Option<usize>,
) {
    let mut block = None;
    push_card_lit(scenes, rect, cw, ch, "crew", |cols, rows| {
        let cells = crate::welcome::welcome_cells_animated(cols, rows, tick, restore);
        block = block_rect(&cells, cols, rows, restore.is_some());
        cells
    });
    let Some(b) = block else {
        return;
    };
    // Whole rows centre the block to half a row, and the card's sub-cell
    // remainder all fell below it: it sat 14px high in the shot window
    // (glass survey D-L9). The block and the lines it holds move by the px
    // that centre it; the stamp keeps the card's last row.
    let at = scenes.len() - 2; // push_framed: the content, then its frame
    let shift = centring(rect, cw, ch, b);
    let (held, rest) = std::mem::take(&mut scenes[at].cells)
        .into_iter()
        .partition(|c| (b.1..b.1 + b.3).contains(&c.row));
    scenes[at].cells = rest;
    let lines = PaneScene {
        cells: held,
        x: scenes[at].x + shift.0,
        y: scenes[at].y + shift.1,
        ..scenes[at].clone()
    };
    push_block(scenes, rect, cw, ch, b, shift);
    scenes.push(lines);
}

/// The whole px that put block `b`'s centre on `rect`'s, held under a cell
/// so the block never leaves the padding `block_rect` keeps round it.
fn centring(rect: Rect, cw: f32, ch: f32, b: (u16, u16, u16, u16)) -> (f32, f32) {
    let off = |at: f32, len: f32, from: u16, n: u16, cell: f32| {
        let mid = at + f32::from(from + 1) * cell + f32::from(n) * cell / 2.0;
        (at + len / 2.0 - mid).round().clamp(1.0 - cell, cell - 1.0)
    };
    (
        off(rect.x, rect.w, b.0, b.2, cw),
        off(rect.y, rect.h, b.1, b.3, ch),
    )
}

/// Blank columns and rows between the block's content and its frame.
const PAD: (u16, u16) = (2, 1);

/// The block card's `(col, row, cols, rows)` in the welcome's interior grid:
/// the rain box, widened to the widest line under it, padded by [`PAD`] and
/// framed. `None` when there is no rain (the one-line banner) or the frame
/// would not fit clear of the card's edges and the version stamp's row.
fn block_rect(
    cells: &[CellView],
    cols: u16,
    rows: u16,
    restore: bool,
) -> Option<(u16, u16, u16, u16)> {
    let (top, left, w, h) = crate::welcome::rain_box(cols, rows, restore)?;
    // The lines under the rain; the stamp's row is the card's, not the block's.
    let under = cells
        .iter()
        .filter(|c| c.row >= top + h && c.row + 1 < rows);
    let (mut c0, mut c1, mut r1) = (left, left + w - 1, top + h - 1);
    for c in under {
        (c0, c1, r1) = (c0.min(c.col), c1.max(c.col), r1.max(c.row));
    }
    let (px, py) = (PAD.0 + 1, PAD.1 + 1);
    let fits = c0 >= px && top >= py && c1 + px < cols && r1 + py + 1 < rows;
    fits.then(|| {
        (
            c0 - px,
            top - py,
            c1 - c0 + 1 + 2 * px,
            r1 - top + 1 + 2 * py,
        )
    })
}

/// The rain block's own card: a lit ring on glass risen above the welcome's
/// (lift 1.5 on its 1), so the field reads as an object on the page rather
/// than glyphs printed onto it. Its sheet is drawn before any cell — every
/// sheet is (`CellGrid::draw`) — so the rain lands on top of it, and the
/// welcome's cells keep the default background, which draws no quad over it.
fn push_block(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    cw: f32,
    ch: f32,
    (col, row, cols, rows): (u16, u16, u16, u16),
    (dx, dy): (f32, f32),
) {
    let t = crew_theme::theme();
    let stroke = crate::panecardglow::focused_stroke(t);
    let mut frame = crate::boxdraw::titled_card(cols, rows, "", stroke, t.ink, t.page_bg);
    crate::modernring::ring(&mut frame, cols, rows, false, 1.0, 0);
    scenes.push(PaneScene {
        cells: frame,
        // The welcome's interior starts one cell in from `rect`.
        x: rect.x + f32::from(col + 1) * cw + dx,
        y: rect.y + f32::from(row + 1) * ch + dy,
        w: f32::from(cols) * cw,
        h: f32::from(rows) * ch,
        focused: true,
        glass: true,
        lift: 1.5,
        ..Default::default()
    });
}

#[cfg(test)]
#[path = "welcomecard_tests.rs"]
mod tests;
