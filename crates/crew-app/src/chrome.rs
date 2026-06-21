//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Padding inset for the input bar card.
pub const INPUT_PAD: f32 = 6.0;

/// Height in physical px reserved for the bottom input bar (≈3 cell rows + padding).
pub fn input_h(cell_h: f32) -> f32 {
    cell_h * 3.0 + INPUT_PAD
}

/// Full-width bottom strip for the docked input bar, inset by `gap` on all sides.
pub fn inputbar_rect(sw: f32, sh: f32, ih: f32, gap: f32) -> Rect {
    Rect {
        x: gap,
        y: sh - ih + gap,
        w: sw - 2.0 * gap,
        h: ih - 2.0 * gap,
    }
}

/// Fixed-width sidebar column on the left, inset by `gap` top/bottom/left so it
/// aligns vertically with the gap-inset grid panes. `ih` is the input-bar height
/// reserved at the bottom.
pub fn sidebar_rect(sh: f32, nav_px: f32, gap: f32, ih: f32) -> Rect {
    Rect {
        x: gap,
        y: gap,
        w: nav_px,
        h: sh - 2.0 * gap - ih,
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
    fn sidebar_rect_inset_by_gap_and_ih() {
        // h = sh - 2*gap - ih = 800 - 16 - 60 = 724
        assert_eq!(
            sidebar_rect(800.0, 200.0, 8.0, 60.0),
            Rect {
                x: 8.0,
                y: 8.0,
                w: 200.0,
                h: 724.0
            }
        );
    }

    #[test]
    fn inputbar_rect_geometry() {
        // x=gap=8, y=sh-ih+gap=800-60+8=748, w=sw-2*gap=1000-16=984, h=ih-2*gap=60-16=44
        assert_eq!(
            inputbar_rect(1000.0, 800.0, 60.0, 8.0),
            Rect {
                x: 8.0,
                y: 748.0,
                w: 984.0,
                h: 44.0
            }
        );
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
