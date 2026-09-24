//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::{Gutter, Rect};

/// Exact height of the bottom chrome: from the input-bar card's cell-quantized
/// top edge ([`card_bottom`] minus its 3 rows) down to the surface bottom.
/// [`content_rect`] subtracts this, so the content area's bottom edge IS the
/// input bar's top — the grid's full outer `gap` then lands the last row of
/// tiles exactly one gap above the bar, the same seam every other card keeps.
/// (The old fixed `3*ch + 2*gap + pad` reserve drifted with the cell-height
/// quantization remainder, so the seam wandered with font size.)
pub fn bottom_chrome_h(sh: f32, ch: f32, gap: impl Into<Gutter>) -> f32 {
    let g: Gutter = gap.into();
    // The tiles' bottom margin (`my`) is added back by the grid; what is left
    // between the last tile and the bar is one `y`.
    sh - (card_bottom(sh, g) - 3.0 * ch) + g.y - g.my
}

/// The bottom y shared by the full-height sidebar and the input-bar card, so
/// their bottom borders land on the exact same pixel row: one margin above
/// the window's edge. Both frames stretch to their rects (`PaneScene::
/// stretch`), so there is no cell remainder left to align away.
pub fn card_bottom(sh: f32, gap: impl Into<Gutter>) -> f32 {
    sh - gap.into().my
}

/// Bottom input-bar card, bottom-aligned to [`card_bottom`] so its bottom border
/// lines up exactly with the sidebar's. Spans the action area width (content
/// x/width, gap-inset). Always a 3-cell-row card.
pub fn inputbar_rect(content: Rect, sh: f32, ch: f32, gap: impl Into<Gutter>) -> Rect {
    let g: Gutter = gap.into();
    let h = 3.0 * ch;
    Rect {
        x: content.x + g.mx,
        y: card_bottom(sh, g) - h,
        w: content.w - 2.0 * g.mx,
        h,
    }
}

/// Fixed-width sidebar column on the left spanning the **entire** height (inset by
/// `gap` on all sides) — it runs alongside both the panes and the input bar.
pub fn sidebar_rect(sh: f32, nav_px: f32, gap: impl Into<Gutter>) -> Rect {
    let g: Gutter = gap.into();
    Rect {
        x: g.mx,
        y: g.my,
        w: nav_px,
        h: sh - 2.0 * g.my,
    }
}

/// Cell rows the docked UPDATE card occupies while a `/update` runs (2 border
/// + 2 content rows).
pub const UPDATE_CARD_ROWS: f32 = 4.0;

/// The nav's top slot: the rect the UPDATE card and the RESTART button share.
/// The same rect `stats_card_rect` shrinks below, and the one the restart
/// click is tested against — one definition, so the button cannot be drawn
/// somewhere the pointer is not looking.
pub fn top_card_rect(sh: f32, nav_px: f32, gap: impl Into<Gutter>, ch: f32) -> Rect {
    let sb = sidebar_rect(sh, nav_px, gap);
    Rect {
        h: (UPDATE_CARD_ROWS * ch).min(sb.h),
        ..sb
    }
}

/// The sidebar stats card's rect: the full column from [`sidebar_rect`],
/// shrunk below the UPDATE card (plus a gap) while an update runs. Shared by
/// drawing (`navcard`) and PANES-row hit-testing (`hit`) so the drawn rows
/// and the click mapping shift together.
pub fn stats_card_rect(
    sh: f32,
    nav_px: f32,
    gap: impl Into<Gutter>,
    ch: f32,
    update: bool,
) -> Rect {
    let g: Gutter = gap.into();
    let sb = sidebar_rect(sh, nav_px, g);
    if !update {
        return sb;
    }
    let h = (UPDATE_CARD_ROWS * ch).min(sb.h);
    Rect {
        y: sb.y + h + g.y,
        h: (sb.h - h - g.y).max(0.0),
        ..sb
    }
}

/// The content area for grid panes: everything to the right of the sidebar. When
/// the sidebar is shown, leave one `gap` of space between it and the first pane
/// (the grid's own internal gap supplies the remaining inset). `ih` is the
/// input-bar height subtracted from the bottom.
pub fn content_rect(
    sw: f32,
    sh: f32,
    show_nav: bool,
    nav_px: f32,
    gap: impl Into<Gutter>,
    ih: f32,
) -> Rect {
    // The grid adds its left margin (`mx`) back: what is left between the
    // nav's rect and the first tile's is one `x`.
    let x = if show_nav { nav_px + gap.into().x } else { 0.0 };
    Rect {
        x,
        y: 0.0,
        w: sw - x,
        h: sh - ih,
    }
}

pub fn point_in(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[cfg(test)]
#[path = "chrome_tests.rs"]
mod tests;
