use super::{btn_for, decode, encode, publish, Btn};

#[test]
fn only_the_hovered_card_lights_a_button() {
    let v = encode(Some((3, Btn::Close)));
    assert_eq!(decode(v, 3), Some(Btn::Close));
    assert_eq!(decode(v, 2), None);
    assert_eq!(decode(v, 0), None);
}

#[test]
fn the_unnumbered_card_is_slot_zero() {
    // Zoomed (or a lone pane): the one card carries no legend number, and
    // slot 0 must still be addressable rather than reading as "nothing".
    let v = encode(Some((0, Btn::Min)));
    assert_eq!(decode(v, 0), Some(Btn::Min));
    assert_eq!(decode(v, 1), None);
}

#[test]
fn nothing_hovered_lights_nothing_anywhere() {
    let v = encode(None);
    for slot in 0..4 {
        assert_eq!(decode(v, slot), None);
    }
}

#[test]
fn the_two_buttons_never_encode_alike() {
    assert_ne!(encode(Some((1, Btn::Min))), encode(Some((1, Btn::Close))));
    assert_ne!(encode(Some((1, Btn::Min))), encode(Some((2, Btn::Min))));
    assert_ne!(encode(Some((0, Btn::Min))), encode(None));
}

/// Sidebar rows get their own change detector; the button one must not
/// answer for them (a pointer sweeping the nav crosses no buttons at all,
/// and would otherwise never repaint).
#[test]
fn the_nav_row_is_tracked_separately_from_the_buttons() {
    super::publish_nav(None);
    assert!(super::publish_nav(Some(0)), "onto the first row");
    assert!(!super::publish_nav(Some(0)), "still on it");
    assert!(super::publish_nav(Some(1)), "onto the next row");
    assert!(super::publish_nav(None), "off the list");
    assert!(!super::publish_nav(None), "and stays off");
}

/// The only test that touches the shared button atomic, so it can never race
/// another one in this binary.
#[test]
fn publish_reports_only_real_changes_and_round_trips() {
    publish(None);
    assert!(publish(Some((1, Btn::Min))), "none -> min is a change");
    assert_eq!(btn_for(1), Some(Btn::Min), "and it is readable");
    assert!(!publish(Some((1, Btn::Min))), "same target is not");
    assert!(publish(Some((1, Btn::Close))), "other button on same card");
    assert!(publish(Some((2, Btn::Close))), "same button on other card");
    assert_eq!(btn_for(1), None, "the card it left goes dark");
    assert!(publish(None), "leaving the button is a change");
    assert_eq!(btn_for(2), None);
}
