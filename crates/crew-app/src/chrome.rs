//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Exact height of the bottom chrome: from the input-bar card's cell-quantized
/// top edge ([`card_bottom`] minus its 3 rows) down to the surface bottom.
/// [`content_rect`] subtracts this, so the content area's bottom edge IS the
/// input bar's top — the grid's full outer `gap` then lands the last row of
/// tiles exactly one gap above the bar, the same seam every other card keeps.
/// (The old fixed `3*ch + 2*gap + pad` reserve drifted with the cell-height
/// quantization remainder, so the seam wandered with font size.)
pub fn bottom_chrome_h(sh: f32, ch: f32, gap: f32) -> f32 {
    sh - (card_bottom(sh, ch, gap) - 3.0 * ch)
}

/// Cell-aligned bottom y shared by the full-height sidebar and the input-bar
/// card, so their bottom borders land on the exact same pixel row. Both are
/// drawn as whole-cell fieldset cards (each floors its height to `floor(h/ch)`
/// rows), so aligning their bottoms requires a common cell-quantized baseline.
pub fn card_bottom(sh: f32, ch: f32, gap: f32) -> f32 {
    gap + ((sh - 2.0 * gap) / ch).floor() * ch
}

/// Bottom input-bar card, bottom-aligned to [`card_bottom`] so its bottom border
/// lines up exactly with the sidebar's. Spans the action area width (content
/// x/width, gap-inset). Always a 3-cell-row card.
pub fn inputbar_rect(content: Rect, sh: f32, ch: f32, gap: f32) -> Rect {
    let h = 3.0 * ch;
    Rect {
        x: content.x + gap,
        y: card_bottom(sh, ch, gap) - h,
        w: content.w - 2.0 * gap,
        h,
    }
}

/// Fixed-width sidebar column on the left spanning the **entire** height (inset by
/// `gap` on all sides) — it runs alongside both the panes and the input bar.
pub fn sidebar_rect(sh: f32, nav_px: f32, gap: f32) -> Rect {
    Rect {
        x: gap,
        y: gap,
        w: nav_px,
        h: sh - 2.0 * gap,
    }
}

/// The rects a sheer window keeps SOLID whatever has focus: crew's own
/// furniture — the input bar and, when it is shown, the left nav.
///
/// Transparency is for the canvas and for the cards you are not reading. The
/// bar you type into is not scenery: a status line, a cwd and a command draft
/// read over a wallpaper are worse in every window than they are in an opaque
/// one, and the nav's cards are the same reading job. At full opacity the list
/// is empty — there is nothing to hand back — which is also what keeps an
/// opaque window's frame byte-identical to the one before this existed.
pub fn solid_chrome(opacity: f32, input_bar: Rect, nav: Option<Rect>) -> Vec<[f32; 4]> {
    if opacity >= 1.0 {
        return Vec::new();
    }
    [Some(input_bar), nav]
        .into_iter()
        .flatten()
        .map(|r| [r.x, r.y, r.w, r.h])
        .collect()
}

/// Cell rows the docked UPDATE card occupies while a `/update` runs (2 border
/// + 2 content rows).
pub const UPDATE_CARD_ROWS: f32 = 4.0;

/// The sidebar stats card's rect: the full column from [`sidebar_rect`],
/// shrunk below the UPDATE card (plus a gap) while an update runs. Shared by
/// drawing (`navcard`) and PANES-row hit-testing (`hit`) so the drawn rows
/// and the click mapping shift together.
pub fn stats_card_rect(sh: f32, nav_px: f32, gap: f32, ch: f32, update: bool) -> Rect {
    let sb = sidebar_rect(sh, nav_px, gap);
    if !update {
        return sb;
    }
    let h = (UPDATE_CARD_ROWS * ch).min(sb.h);
    Rect {
        y: sb.y + h + gap,
        h: (sb.h - h - gap).max(0.0),
        ..sb
    }
}

/// The content area for grid panes: everything to the right of the sidebar. When
/// the sidebar is shown, leave one `gap` of space between it and the first pane
/// (the grid's own internal gap supplies the remaining inset). `ih` is the
/// input-bar height subtracted from the bottom.
pub fn content_rect(sw: f32, sh: f32, show_nav: bool, nav_px: f32, gap: f32, ih: f32) -> Rect {
    let x = if show_nav { nav_px + gap } else { 0.0 };
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
