//! The weather marks fill their cell like the text beside them.
use crate::boxglyph::synth;

/// Inked extent `(width, height)` of `c` drawn in a `w`×`h` cell.
fn extent(c: char, w: u32, h: u32) -> (u32, u32) {
    let img = synth(c, w, h, 12).unwrap_or_else(|| panic!("{c:?} not drawn"));
    let (iw, ih) = (img.placement.width, img.placement.height);
    let on = |x: u32, y: u32| img.data[(y * iw + x) as usize] > 96;
    let cols = (0..iw).filter(|&x| (0..ih).any(|y| on(x, y))).count() as u32;
    let rows = (0..ih).filter(|&y| (0..iw).any(|x| on(x, y))).count() as u32;
    (cols, rows)
}

#[test]
fn every_weather_mark_is_drawn_at_the_cells_width() {
    for c in ['\u{2600}', '\u{2601}', '\u{2602}', '\u{2744}'] {
        let (w, h) = extent(c, 10, 20);
        // The cloud was a hump a third of a cell tall; the umbrella a speck.
        // Drawn, each is most of the cell wide and about a capital tall.
        assert!(w >= 7, "{c:?} is {w} px wide in a 10 px cell");
        assert!(h >= 6, "{c:?} is {h} px tall in a 20 px cell");
    }
    // The bolt is a double-width character: its box is two cells.
    let (w, h) = extent('\u{26A1}', 20, 20);
    assert!(w >= 8 && h >= 9, "bolt {w}x{h}");
}

#[test]
fn the_umbrella_has_a_canopy_and_a_handle_below_it() {
    let img = synth('\u{2602}', 10, 20, 12).unwrap();
    let w = img.placement.width;
    let at = |x: u32, y: u32| img.data[(y * w + x) as usize];
    let (cx, cy) = (5u32, 10u32);
    // Canopy spans wide just above the centre; the handle is thin below.
    let canopy = (0..w).filter(|&x| at(x, cy - 1) > 96).count();
    let handle = (0..w).filter(|&x| at(x, cy + 3) > 96).count();
    assert!(
        canopy >= 7 && (1..=3).contains(&handle),
        "canopy {canopy}, handle {handle}"
    );
    assert!(at(cx, cy + 3) > 96, "the handle stands under the middle");
}
