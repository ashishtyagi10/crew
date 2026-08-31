use super::{icon, Over};

/// Every state must be told apart from the ones next to it — an icon table
/// where two verbs share a shape says nothing.
/// `Link` and `Control` deliberately SHARE the hand: both are "this does
/// something when you press it", and a third shape for the distinction
/// would be one nobody has learned. Everything else must be distinct.
#[test]
fn a_link_wears_the_same_hand_a_control_does() {
    assert_eq!(icon(Over::Link), icon(Over::Control));
    assert_ne!(icon(Over::Link), icon(Over::Text));
}

#[test]
fn every_state_has_its_own_shape() {
    let all = [
        Over::Carrying,
        Over::NavEdge,
        Over::Gutter,
        Over::Control,
        Over::Handle,
        Over::Text,
        Over::Page,
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(icon(*a), icon(*b), "{a:?} and {b:?} look the same");
        }
    }
}

/// A card already in hand reads as held, not as grabbable.
#[test]
fn carrying_and_grabbing_are_not_the_same_shape() {
    assert_eq!(icon(Over::Handle), winit::window::CursorIcon::Grab);
    assert_eq!(icon(Over::Carrying), winit::window::CursorIcon::Grabbing);
}

#[test]
fn the_bare_page_leaves_the_pointer_alone() {
    assert_eq!(icon(Over::Page), winit::window::CursorIcon::Default);
}
