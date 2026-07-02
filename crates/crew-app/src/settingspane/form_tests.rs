use ratatui::layout::{Position, Rect};

use super::form::{layout, scroll_for, STACK_BELOW};
use super::{Field, FIELDS};

#[test]
fn wide_pane_lays_out_two_columns() {
    let lay = layout(80);
    assert_eq!(lay.cards.len(), 3);
    let appearance = &lay.cards[0];
    let window = &lay.cards[1];
    let notifications = &lay.cards[2];
    assert_eq!(appearance.title, "APPEARANCE");
    // Window sits in the right column, Notifications stacked below it.
    assert!(window.rect.x > appearance.rect.x);
    assert_eq!(notifications.rect.x, window.rect.x);
    assert!(notifications.rect.y >= window.rect.y + window.rect.height);
    assert_eq!(
        lay.height,
        lay.cards
            .iter()
            .map(|c| c.rect.y + c.rect.height)
            .max()
            .unwrap()
    );
}

#[test]
fn narrow_pane_stacks_single_column() {
    let lay = layout(STACK_BELOW - 1);
    let xs: Vec<u16> = lay.cards.iter().map(|c| c.rect.x).collect();
    assert!(xs.iter().all(|&x| x == xs[0]), "same x: {xs:?}");
    for w in lay.cards.windows(2) {
        assert!(w[1].rect.y >= w[0].rect.y + w[0].rect.height);
    }
}

#[test]
fn every_form_field_has_a_rect() {
    let lay = layout(80);
    for f in FIELDS.iter().take(FIELDS.len() - 2) {
        assert!(lay.rect_of(*f).is_some(), "{f:?} missing a rect");
    }
    // Buttons are pinned outside the scrolled form.
    assert!(lay.rect_of(Field::Save).is_none());
}

#[test]
fn field_rects_stay_inside_their_card() {
    let lay = layout(80);
    for (f, r) in &lay.rects {
        assert!(
            lay.cards
                .iter()
                .any(|c| c.rect.contains(Position::new(r.x, r.y))
                    && r.y + r.height <= c.rect.y + c.rect.height),
            "{f:?} rect {r:?} escapes every card"
        );
    }
}

#[test]
fn scroll_for_keeps_the_focused_rect_visible() {
    // Everything fits → no scroll.
    assert_eq!(scroll_for(Rect::new(0, 5, 10, 3), 20, 30), 0);
    // Focus near the bottom → scrolls just enough to show its bottom edge.
    assert_eq!(scroll_for(Rect::new(0, 20, 10, 3), 25, 10), 13);
    // Focus at the top → back to zero.
    assert_eq!(scroll_for(Rect::new(0, 0, 10, 3), 25, 10), 0);
    // Never past the end.
    assert_eq!(scroll_for(Rect::new(0, 24, 10, 1), 25, 10), 15);
}
