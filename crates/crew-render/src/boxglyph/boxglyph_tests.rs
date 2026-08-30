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
    for c in ['a', 'W', '·', '●', '╱', '╪', '╫'] {
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

/// A double line is two strokes with a gap: three transitions across the
/// cell, not one. Anything that draws it as a single thick bar has lost the
/// distinction the character exists to make.
#[test]
fn a_double_rule_is_two_strokes_with_a_gap() {
    let img = cell('\u{2550}', 8, 16);
    let col: Vec<u8> = (0..16).map(|y| at(&img, 4, y)).collect();
    let runs = col.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(runs, 4, "═ crossed as {col:?}");
    let img = cell('\u{2551}', 8, 16);
    let row: Vec<u8> = (0..8).map(|x| at(&img, x, 8)).collect();
    assert_eq!(
        row.windows(2).filter(|w| w[0] != w[1]).count(),
        4,
        "{row:?}"
    );
}

/// The corners are the hard half of the double set: at a turn the OUTER
/// stroke of each arm runs past the far side of the other's band and the
/// INNER stroke stops at its near side. Get it wrong and the corner comes
/// out as a lattice with its corner missing.
#[test]
fn double_corners_close_on_the_outside_and_open_on_the_inside() {
    let (w, h) = (9u32, 17u32);
    for (c, right, down) in [
        ('\u{2554}', true, true),
        ('\u{2557}', false, true),
        ('\u{255A}', true, false),
        ('\u{255D}', false, false),
    ] {
        let img = cell(c, w, h);
        // Two strokes leave along each present arm, none along the absent.
        let edge = |x: u32| (0..h).filter(|y| at(&img, x, *y) == 255).count();
        let side = |y: u32| (0..w).filter(|x| at(&img, *x, y) == 255).count();
        assert_eq!(edge(if right { w - 1 } else { 0 }), 2, "{c:?} arm strokes");
        assert_eq!(side(if down { h - 1 } else { 0 }), 2, "{c:?} arm strokes");
        assert_eq!(edge(if right { 0 } else { w - 1 }), 0, "{c:?} spurious");
        assert_eq!(side(if down { 0 } else { h - 1 }), 0, "{c:?} spurious");
        // The outer stroke turns THROUGH the far side of the other band: the
        // corner pixel where the two outer strokes meet carries ink. Without
        // it the corner is two lines that stop short of each other.
        // The two bands sit at columns 3 and 5, rows 7 and 9 at this size.
        let (cx, cy) = (if right { 3 } else { 5 }, if down { 7 } else { 9 });
        assert_eq!(at(&img, cx, cy), 255, "{c:?} outer corner is open");
        // …and the gap between the strokes is not filled in: that gap is
        // what makes it a DOUBLE rather than a thick single.
        assert_eq!(at(&img, 4, 8), 0, "{c:?} inner corner is filled in");
    }
}

/// A T-junction's inner stroke steps aside for the branch rather than
/// walling it off: `╠`'s right vertical is broken where the two horizontals
/// leave it, `╦`'s lower horizontal where the two verticals descend.
#[test]
fn double_tees_open_for_their_branch() {
    let tee = cell('\u{2560}', 9, 17);
    assert_eq!(at(&tee, 5, 8), 0, "╠ walls its own branch off");
    assert_eq!(at(&tee, 3, 8), 255, "…but the outer stroke runs through");
    let tee = cell('\u{2566}', 9, 17);
    assert_eq!(at(&tee, 4, 9), 0, "╦ walls its own branch off");
    assert_eq!(at(&tee, 4, 7), 255, "…but the outer stroke runs through");
}

/// A cross must leave a hole — that is the whole shape of `╬`, and what
/// tells it apart from a `┼` drawn thick. The four strokes stop at the far
/// edge of the band they meet, so the junction is four corners around an
/// empty square, not a lattice.
#[test]
fn the_double_cross_keeps_its_hole() {
    let img = cell('\u{256C}', 9, 17);
    for y in 7..10 {
        for x in 3..6 {
            assert_eq!(at(&img, x, y), 0, "╬ is solid at ({x}, {y})");
        }
    }
    // And the arms still arrive: two strokes on each of the four sides.
    assert_eq!((0..17).filter(|y| at(&img, 0, *y) == 255).count(), 2);
    assert_eq!((0..9).filter(|x| at(&img, *x, 0) == 255).count(), 2);
}

/// Print every junction as ASCII art. Not an assertion — a tool: two of the
/// double set's junction rules were wrong in ways every geometric assertion
/// above still passed, and reading the masks out is how they were found.
#[test]
#[ignore = "prints the masks; run it to look at a junction"]
fn print_the_junctions() {
    for c in [
        '\u{2554}', '\u{2557}', '\u{255A}', '\u{255D}', '\u{256C}', '\u{2560}', '\u{2566}',
    ] {
        let img = cell(c, 9, 17);
        println!("--- {c} ---");
        for y in 0..17 {
            let row: String = (0..9)
                .map(|x| if at(&img, x, y) > 128 { '#' } else { '.' })
                .collect();
            println!("{row}");
        }
    }
}

/// A dashed rule is its solid sibling with the ink taken out in even steps:
/// same stroke, same centre line, more marks the higher its number.
#[test]
fn dashed_rules_break_where_they_should() {
    let solid = cell('\u{2500}', 24, 16);
    let row = (0..16).find(|y| at(&solid, 0, *y) == 255).expect("no rule");
    for (c, marks) in [('\u{254C}', 2), ('\u{2504}', 3), ('\u{2508}', 4)] {
        let img = cell(c, 24, 16);
        let runs = (0..24)
            .map(|x| at(&img, x, row))
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] == 0 && w[1] == 255)
            .count();
        // The first mark starts at the edge, so it opens no run of its own.
        assert_eq!(runs + 1, marks, "{c:?} drew {} marks", runs + 1);
        assert_eq!(at(&img, 0, row), 255, "{c:?} must meet its neighbour");
    }
    let heavy = cell('\u{2505}', 24, 16);
    let light = cell('\u{2504}', 24, 16);
    let ink = |i: &SwashImage| i.data.iter().filter(|v| **v > 0).count();
    assert!(ink(&heavy) > ink(&light), "┅ is not heavier than ┄");
    let vert = cell('\u{2506}', 24, 16);
    let col = (0..24).find(|x| at(&vert, *x, 0) == 255).expect("no rule");
    assert!((0..16).any(|y| at(&vert, col, y) == 0), "┆ never breaks");
}
