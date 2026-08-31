use super::{on_edge, width_at, GRAB, MAX_W, MIN_W};

#[test]
fn the_edge_is_grabbable_from_either_side() {
    assert!(on_edge(210.0, 210.0), "dead on it");
    assert!(on_edge(210.0 - GRAB, 210.0), "just inside the nav");
    assert!(on_edge(210.0 + GRAB, 210.0), "just inside the content");
    assert!(!on_edge(210.0 - GRAB - 1.0, 210.0));
    assert!(!on_edge(210.0 + GRAB + 1.0, 210.0));
}

/// The edge follows the nav, so it is grabbable wherever the nav is —
/// including after a previous drag moved it.
#[test]
fn the_edge_moves_with_the_nav() {
    assert!(on_edge(160.0, 160.0));
    assert!(on_edge(320.0, 320.0));
    assert!(!on_edge(210.0, 160.0), "not where it used to be");
}

/// A drag off the side of the window must not collapse the nav to nothing
/// or run it across the whole canvas — the same bounds the Settings form
/// clamps its typed figure to.
#[test]
fn a_drag_cannot_take_the_nav_outside_its_bounds() {
    assert_eq!(width_at(0.0), MIN_W);
    assert_eq!(width_at(-500.0), MIN_W);
    assert_eq!(width_at(9_000.0), MAX_W);
    assert_eq!(width_at(240.0), 240.0, "and leaves a legal width alone");
}
