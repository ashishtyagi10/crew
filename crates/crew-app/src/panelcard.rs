//! Fieldset card for a *non-pane* panel (sidebar, welcome, command menu,
//! update card): an inset content buffer plus a dim border card carrying the
//! legend, pushed as two [`PaneScene`]s so the border never shifts the content.
//! The pane version (with focus, status glyphs) lives in
//! [`crate::panecard`]; this is the plain box on the one canvas.
use crew_render::{CellView, PaneScene};

use crate::boxdraw::titled_card;
use crate::layout::Rect;

/// Push a fieldset card into `scenes`: `content` builds the interior cells at
/// the inset `(cols, rows)` grid; a dim `legend`-titled border frames it.
pub fn push_card(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    cw: f32,
    ch: f32,
    legend: &str,
    content: impl FnOnce(u16, u16) -> Vec<CellView>,
) {
    let (icols, irows) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    scenes.push(PaneScene {
        cells: content(icols, irows),
        x: rect.x + cw,
        y: rect.y + ch,
        w: (rect.w - 2.0 * cw).max(0.0),
        h: (rect.h - 2.0 * ch).max(0.0),
        focused: false,
        bordered: false,
        overlay: false,
    });
    scenes.push(PaneScene {
        cells: titled_card(
            icols + 2,
            irows + 2,
            legend,
            crew_theme::theme().border_normal,
            crew_theme::theme().legend_off,
            crew_theme::theme().page_bg,
        ),
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        focused: false,
        bordered: false,
        overlay: false,
    });
}
