//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Fixed-width, full-height sidebar column on the left.
pub fn sidebar_rect(sw: f32, sh: f32, nav_px: f32) -> Rect {
    let _ = sw;
    Rect {
        x: 0.0,
        y: 0.0,
        w: nav_px,
        h: sh,
    }
}

/// The content area for grid panes: everything to the right of the sidebar.
pub fn content_rect(sw: f32, sh: f32, show_nav: bool, nav_px: f32) -> Rect {
    let x = if show_nav { nav_px } else { 0.0 };
    Rect {
        x,
        y: 0.0,
        w: sw - x,
        h: sh,
    }
}

pub fn point_in(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rect_no_nav() {
        assert_eq!(
            content_rect(1000.0, 800.0, false, 200.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 800.0
            }
        );
    }

    #[test]
    fn content_rect_with_nav() {
        assert_eq!(
            content_rect(1000.0, 800.0, true, 200.0),
            Rect {
                x: 200.0,
                y: 0.0,
                w: 800.0,
                h: 800.0
            }
        );
    }

    #[test]
    fn sidebar_rect_full_height() {
        assert_eq!(
            sidebar_rect(1000.0, 800.0, 200.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 800.0
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
