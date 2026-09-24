//! One gutter between every two cards' LINES, one margin at every window
//! edge — measured the way the eye measures it, stroke to stroke.
use super::*;
use crate::chrome::{bottom_chrome_h, content_rect, inputbar_rect, sidebar_rect};

/// The drawn stroke box of a card at `r`: each rule `inset` into its rect.
fn strokes(r: Rect, (ix, iy): (f32, f32)) -> (f32, f32, f32, f32) {
    (r.x + ix, r.y + iy, r.x + r.w - ix, r.y + r.h - iy)
}

/// Nav, a 2+1 grid of tiles and the input bar, at cells from a small font
/// to a Retina one — every seam between two lines is the same gutter, and
/// every line to the window's edge is that same distance.
#[test]
fn every_seam_and_margin_is_one_gutter() {
    for (cw, ch) in [(7.8_f32, 16.0_f32), (8.4, 17.5), (12.6, 26.0), (16.8, 34.0)] {
        let inset = (cw / 2.0, ch / 2.0);
        let g = Gutter::between_strokes(8.0, inset);
        let want = 8.0 + cw / 2.0;
        let (sw, sh, nav) = (1512.0_f32, 945.0_f32, 60.0_f32);
        let sb = strokes(sidebar_rect(sh, nav, g), inset);
        let content = content_rect(sw, sh, true, nav, g, bottom_chrome_h(sh, ch, g));
        let tiles: Vec<_> = pane_rects_at(3, content.x, content.y, content.w, content.h, g)
            .into_iter()
            .map(|r| strokes(r, inset))
            .collect();
        let ib = strokes(inputbar_rect(content, sh, ch, g), inset);
        let near = |got: f32, what: &str| {
            assert!(
                (got - want).abs() <= 1.0,
                "cell {cw}x{ch}: {what} is {got}, want {want}"
            )
        };
        near(sb.0, "left margin");
        near(sb.1, "top margin");
        near(sh - sb.3, "bottom margin");
        near(tiles[0].1, "tiles' top margin");
        near(sw - tiles[2].2, "right margin");
        near(tiles[0].0 - sb.2, "nav → tile");
        near(tiles[2].0 - tiles[0].2, "tile → tile, across");
        near(tiles[1].1 - tiles[0].3, "tile → tile, down");
        near(ib.1 - tiles[1].3, "tile → input bar");
        near(ib.1 - tiles[2].3, "tall tile → input bar");
        near(ib.0 - sb.2, "nav → input bar");
        assert!(
            (sb.3 - ib.3).abs() < 0.01,
            "nav and input bar share a bottom line"
        );
    }
}

/// A bare gap is the old even spacing: every rect gap the same.
#[test]
fn a_bare_gap_is_even() {
    assert_eq!(
        Gutter::from(8.0),
        Gutter {
            x: 8.0,
            y: 8.0,
            mx: 8.0,
            my: 8.0
        }
    );
}
