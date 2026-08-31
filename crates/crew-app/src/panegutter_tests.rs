use super::gutter_frac;

/// Top of the interior is the top of the buffer, bottom is the live edge.
#[test]
fn the_gutter_maps_the_cards_interior_to_the_whole_buffer() {
    let (y, h, ch) = (100.0, 200.0, 20.0);
    assert_eq!(gutter_frac(y, h, ch, 120.0), Some(0.0), "first inner row");
    assert_eq!(gutter_frac(y, h, ch, 280.0), Some(1.0), "last inner row");
    let mid = gutter_frac(y, h, ch, 200.0).unwrap();
    assert!((mid - 0.5).abs() < 0.01, "halfway down is {mid}");
}

/// A pointer dragged off the top or bottom of the card pins to an end
/// rather than running the offset past the buffer.
#[test]
fn dragging_past_either_end_pins_instead_of_overshooting() {
    assert_eq!(gutter_frac(100.0, 200.0, 20.0, -900.0), Some(0.0));
    assert_eq!(gutter_frac(100.0, 200.0, 20.0, 9_000.0), Some(1.0));
}

/// A card with no room between its borders has no gutter at all — the
/// division that maps the interior would be by zero.
#[test]
fn a_card_with_no_interior_has_no_gutter() {
    assert_eq!(gutter_frac(0.0, 40.0, 20.0, 20.0), None);
    assert_eq!(gutter_frac(0.0, 10.0, 20.0, 5.0), None);
}
