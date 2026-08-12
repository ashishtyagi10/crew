use super::*;

/// The weave is what the modern family is after: a fine SQUARE grid, about
/// six dots to a text row, not a sparse polka grid on a cell multiple.
#[test]
fn cell_geometry_is_a_square_grid_about_six_dots_to_a_row() {
    let (spacing, _) = DotLattice::cell_geometry(36.0); // retina, ~14pt
    assert_eq!(spacing[0], spacing[1], "lattice must be square");
    assert_eq!(spacing[0], 6.0);
    assert!(
        36.0 / spacing[1] >= 5.0,
        "want at least five dots per text row, got {}",
        36.0 / spacing[1]
    );
}

/// Dots stay pin-fine: even with the shader's ±0.8px feather on each side,
/// neighbouring dots leave clear page between them rather than merging into
/// a flat tint.
#[test]
fn dots_never_touch_their_neighbours() {
    for cell_h in [12.0f32, 20.0, 28.0, 36.0, 44.0, 72.0, 200.0] {
        let (spacing, radius) = DotLattice::cell_geometry(cell_h);
        let painted = 2.0 * (radius + 0.8); // feathered diameter
        assert!(
            painted < spacing[0],
            "cell_h {cell_h}: painted {painted} >= pitch {}",
            spacing[0]
        );
    }
}

/// The pitch rides the cell (font size and DPI) inside the working band, and
/// clamps outside it so extreme fonts still land on a pitch that renders
/// cleanly.
#[test]
fn pitch_rides_the_cell_and_clamps_at_the_extremes() {
    assert_eq!(DotLattice::cell_geometry(48.0).0[0], 8.0);
    assert_eq!(DotLattice::cell_geometry(36.0).0[0], 6.0);
    // Tiny cell: 12/6 = 2 would alias — floor at 5.
    assert_eq!(DotLattice::cell_geometry(12.0).0[0], 5.0);
    // Huge cell: 200/6 = 33 would read as polka dots — ceiling at 9.
    assert_eq!(DotLattice::cell_geometry(200.0).0[0], 9.0);
}

#[test]
fn radius_is_a_fifth_of_the_pitch_within_its_own_clamps() {
    assert_eq!(DotLattice::cell_geometry(36.0).1, 1.2);
    assert_eq!(DotLattice::cell_geometry(12.0).1, 1.0); // pitch 5 → 1.0
    assert_eq!(DotLattice::cell_geometry(200.0).1, 1.4); // pitch 9 → 1.8, capped
}
