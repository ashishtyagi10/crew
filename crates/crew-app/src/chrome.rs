//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Padding inset for the input bar card.
pub const INPUT_PAD: f32 = 6.0;

/// Height in physical px reserved for the bottom input bar. Must yield at least 3
/// cell rows AFTER the `2*gap` inset (the card needs top border + input + bottom
/// border), so reserve `3 rows + 2*gap + pad`.
pub fn input_h(cell_h: f32) -> f32 {
    cell_h * 3.0 + 2.0 * crate::app::GAP + INPUT_PAD
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
mod tests {
    use super::*;

    #[test]
    fn content_rect_no_nav_with_ih() {
        // h = sh - ih = 800 - 60 = 740
        assert_eq!(
            content_rect(1000.0, 800.0, false, 200.0, 8.0, 60.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 740.0
            }
        );
    }

    #[test]
    fn content_rect_with_nav_with_ih() {
        // x = nav_px + gap = 208; w = 1000 - 208 = 792; h = 800 - 60 = 740
        assert_eq!(
            content_rect(1000.0, 800.0, true, 200.0, 8.0, 60.0),
            Rect {
                x: 208.0,
                y: 0.0,
                w: 792.0,
                h: 740.0
            }
        );
    }

    #[test]
    fn sidebar_rect_full_height() {
        // full height: h = sh - 2*gap = 800 - 16 = 784 (input bar does NOT shrink it)
        assert_eq!(
            sidebar_rect(800.0, 200.0, 8.0),
            Rect {
                x: 8.0,
                y: 8.0,
                w: 200.0,
                h: 784.0
            }
        );
    }

    #[test]
    fn inputbar_rect_spans_action_area() {
        // content (with nav) = {x:208, w:792}; x and w are unchanged.
        // ch=20: card_bottom(800,20,8) = 8 + floor(784/20)*20 = 8+780 = 788;
        // h = 3*20 = 60; y = 788-60 = 728.
        let content = content_rect(1000.0, 800.0, true, 200.0, 8.0, 60.0);
        assert_eq!(
            inputbar_rect(content, 800.0, 20.0, 8.0),
            Rect {
                x: 216.0,
                y: 728.0,
                w: 776.0,
                h: 60.0
            }
        );
    }

    #[test]
    fn sidebar_and_inputbar_bottoms_align() {
        // Fractional cell height (font 14 -> ch 17.5) used to leave the sidebar's
        // floored bottom border above the input bar's bottom. Their drawn bottom
        // borders must now land on the exact same pixel row.
        let (sw, sh, ch, gap, nav) = (1000.0_f32, 800.0_f32, 17.5_f32, 8.0_f32, 200.0_f32);
        let sb = sidebar_rect(sh, nav, gap);
        // push_card draws floor(h/ch) rows starting at sb.y; bottom-border bottom edge:
        let sb_bottom = sb.y + (sb.h / ch).floor() * ch;
        let content = content_rect(sw, sh, true, nav, gap, input_h(ch));
        let ib = inputbar_rect(content, sh, ch, gap);
        let ib_bottom = ib.y + (ib.h / ch).floor() * ch;
        assert_eq!(
            sb_bottom, ib_bottom,
            "sidebar and input-bar card bottoms must align"
        );
    }

    #[test]
    fn stats_card_rect_shifts_below_the_update_card() {
        // No update: the stats card IS the sidebar column.
        let sb = sidebar_rect(800.0, 200.0, 8.0);
        assert_eq!(stats_card_rect(800.0, 200.0, 8.0, 20.0, false), sb);
        // Update running: shifted down by the 4-row card plus one gap, and
        // shrunk by the same amount.
        let shifted = stats_card_rect(800.0, 200.0, 8.0, 20.0, true);
        assert_eq!(shifted.y, sb.y + 4.0 * 20.0 + 8.0);
        assert_eq!(shifted.h, sb.h - 4.0 * 20.0 - 8.0);
        assert_eq!((shifted.x, shifted.w), (sb.x, sb.w));
    }

    #[test]
    fn point_in_bounds() {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            w: 30.0,
            h: 30.0,
        };
        assert!(point_in(r, 5.0, 5.0));
        assert!(!point_in(r, 100.0, 5.0));
    }
}
