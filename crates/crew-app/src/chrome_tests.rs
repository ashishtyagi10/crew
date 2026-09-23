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
    let content = content_rect(sw, sh, true, nav, gap, bottom_chrome_h(sh, ch, gap));
    let ib = inputbar_rect(content, sh, ch, gap);
    let ib_bottom = ib.y + (ib.h / ch).floor() * ch;
    assert_eq!(
        sb_bottom, ib_bottom,
        "sidebar and input-bar card bottoms must align"
    );
}

#[test]
fn grid_sits_exactly_one_gap_above_the_input_bar() {
    // The seam between the grid's bottom tile row and the input bar must
    // be the SAME one-gap rhythm as every other seam on the canvas — at
    // every font size, including fractional cell heights. The old
    // `3*ch + 2*gap + pad` reserve let it wander with the cell-height
    // quantization remainder (2px at ch=20, 22px just past a boundary).
    for ch in [16.0_f32, 17.5, 20.0, 21.0, 24.0] {
        let (sw, sh, gap, nav) = (1000.0_f32, 800.0_f32, 8.0_f32, 200.0_f32);
        let content = content_rect(sw, sh, true, nav, gap, bottom_chrome_h(sh, ch, gap));
        let tile =
            crate::layout::pane_rects_at(1, content.x, content.y, content.w, content.h, gap)[0];
        let ib = inputbar_rect(content, sh, ch, gap);
        let seam = ib.y - (tile.y + tile.h);
        assert!(
            (seam - gap).abs() <= 0.5,
            "ch={ch}: grid→input-bar seam is {seam}px, want one gap ({gap}px)"
        );
    }
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
