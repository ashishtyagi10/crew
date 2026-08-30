use super::*;
use crate::boxglyph::arms::arms_of;

/// Coverage of the pixel at `(x, y)` in a synthesized cell.
fn at(img: &SwashImage, x: u32, y: u32) -> u8 {
    img.data[(y * img.placement.width + x) as usize]
}

fn cell(c: char, w: u32, h: u32) -> SwashImage {
    synth(c, w, h, 12).unwrap_or_else(|| panic!("{c:?} not synthesized"))
}

/// The whole point: a rule is either fully on or fully off. Anything in
/// between is the fringe this module replaced, and a horizontal rule has a
/// straight edge, so there is nothing for antialiasing to do.
#[test]
fn a_rule_has_no_partial_pixels() {
    for c in ['─', '│', '┼', '█', '▍', '▁'] {
        let img = cell(c, 8, 16);
        let soft = img.data.iter().filter(|v| **v > 0 && **v < 255).count();
        assert_eq!(soft, 0, "{c:?} drew {soft} partially-covered pixels");
    }
}

/// A `─` is one stroke, and it is where the `│`'s stroke crosses — otherwise
/// a `├` would not meet the `│` above it.
#[test]
fn the_horizontal_and_vertical_rules_share_one_centre() {
    let (w, h) = (8, 16);
    let (hor, ver) = (cell('─', w, h), cell('│', w, h));
    let row = (0..h).find(|y| at(&hor, 0, *y) == 255).expect("no rule");
    let col = (0..w).find(|x| at(&ver, *x, 0) == 255).expect("no rule");
    // Every cell of the rule is inked across its whole length.
    assert!((0..w).all(|x| at(&hor, x, row) == 255), "rule breaks up");
    assert!((0..h).all(|y| at(&ver, col, y) == 255), "rule breaks up");
    let cross = cell('┼', w, h);
    assert_eq!(at(&cross, 0, row), 255, "┼ misses the ─ it must meet");
    assert_eq!(at(&cross, col, 0), 255, "┼ misses the │ it must meet");
}

/// A corner's arms have to run to the edges the neighbouring cells arrive
/// at, or the frame has gaps at all four corners.
#[test]
fn corners_reach_the_edges_their_arms_point_at() {
    let (w, h) = (8, 16);
    for (c, right, down) in [
        ('┌', true, true),
        ('┐', false, true),
        ('└', true, false),
        ('┘', false, false),
        ('╭', true, true),
        ('╮', false, true),
        ('╰', true, false),
        ('╯', false, false),
    ] {
        let img = cell(c, w, h);
        let edge = |x: u32| (0..h).map(|y| at(&img, x, y)).max().unwrap();
        let side = |y: u32| (0..w).map(|x| at(&img, x, y)).max().unwrap();
        assert_eq!(edge(if right { w - 1 } else { 0 }), 255, "{c:?} arm short");
        assert_eq!(side(if down { h - 1 } else { 0 }), 255, "{c:?} arm short");
        assert_eq!(edge(if right { 0 } else { w - 1 }), 0, "{c:?} spurious arm");
        assert_eq!(side(if down { 0 } else { h - 1 }), 0, "{c:?} spurious arm");
    }
}

/// Eighth blocks are the meters; each must be its own fraction of the cell,
/// measured in whole pixels, and they must be strictly ordered.
#[test]
fn eighth_blocks_step_in_whole_pixels() {
    let h = 16;
    let inked = |c: char| {
        let img = cell(c, 8, h);
        (0..h).filter(|y| at(&img, 0, *y) == 255).count()
    };
    let rows: Vec<usize> = "▁▂▃▄▅▆▇█".chars().map(inked).collect();
    assert_eq!(rows, vec![2, 4, 6, 8, 10, 12, 14, 16], "{rows:?}");
    let cols: Vec<usize> = "▏▎▍▌▋▊▉█"
        .chars()
        .map(|c| {
            let img = cell(c, 8, h);
            (0..8).filter(|x| at(&img, *x, 0) == 255).count()
        })
        .collect();
    assert_eq!(cols, vec![1, 2, 3, 4, 5, 6, 7, 8], "{cols:?}");
}

/// Heavy lines have to read heavier than light ones, or the distinction the
/// characters exist to make is lost.
#[test]
fn heavy_lines_are_thicker_than_light_ones() {
    let ink = |c: char, h: u32| cell(c, 8, h).data.iter().filter(|v| **v > 0).count();
    for h in [16, 32] {
        assert!(ink('━', h) > ink('─', h), "━ is not heavier at h={h}");
        assert!(ink('┃', h) > ink('│', h), "┃ is not heavier at h={h}");
    }
}

/// The stroke tracks the cell, so a Retina rescale or a display font size
/// gets a proportionally thicker rule rather than a lone hairline.
#[test]
fn the_stroke_thickens_with_the_cell() {
    assert_eq!(light_thickness(16), 1);
    assert_eq!(light_thickness(32), 2);
    assert_eq!(light_thickness(48), 3);
    assert_eq!(light_thickness(4), 1, "never vanishes");
}

/// Shades are a flat tint at a known fraction — a heatmap reads its value
/// off them.
#[test]
fn shades_are_flat_and_ordered() {
    let level = |c: char| {
        let img = cell(c, 8, 16);
        assert!(img.data.iter().all(|v| *v == img.data[0]), "{c:?} not flat");
        img.data[0]
    };
    assert_eq!((level('░'), level('▒'), level('▓')), (64, 128, 191));
}

/// Characters outside the drawn set must fall through to the font — this
/// module claiming a letter would replace it with a rectangle.
#[test]
fn only_the_drawn_set_is_claimed() {
    for c in ['a', 'W', '·', '●', '╱', '═', '║', '┄'] {
        assert!(synth(c, 8, 16, 12).is_none(), "{c:?} was claimed");
    }
    assert!(arms_of('─').is_some());
    assert!(arms_of('a').is_none());
}

/// A cell too small to hold a rule is left to the font rather than filled
/// with a smear.
#[test]
fn degenerate_cells_are_declined() {
    assert!(synth('─', 1, 16, 12).is_none());
    assert!(synth('─', 8, 1, 12).is_none());
}

/// The mask must cover exactly the cell it was laid into: full width, full
/// height, and anchored so its top edge is `top` above the baseline.
#[test]
fn the_mask_is_exactly_the_cell_box() {
    let img = cell('█', 9, 17);
    assert_eq!(
        (
            img.placement.left,
            img.placement.top,
            img.placement.width,
            img.placement.height
        ),
        (0, 12, 9, 17)
    );
    assert_eq!(img.data.len(), 9 * 17);
}
