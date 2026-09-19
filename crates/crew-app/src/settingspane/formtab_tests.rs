//! How Tab walks the settings form — the ORDER the fields are reached in,
//! split from the geometry tests beside them when the balanced columns made
//! both halves longer than the file could hold.
use super::form::{layout, tab_order};
use super::{Field, FIELDS};

#[test]
fn the_tab_order_is_the_order_the_form_is_drawn_in() {
    // It used to be a hand-written list that had drifted: `Paper grain` is
    // drawn third, beside `Font size`, and was tabbed seventeenth; `Nav width`
    // sits in the WINDOW card and was tabbed fifth, so Tab left the appearance
    // card, crossed the form and came back.
    for cols in [60, 100, 160] {
        let drawn: Vec<Field> = layout(cols).rects.iter().map(|(f, _)| *f).collect();
        let order = tab_order(cols);
        assert_eq!(order[..drawn.len()], drawn[..], "at {cols} cols");
        // The buttons are drawn by `render`, not placed by `layout`, so they
        // arrive through the fallback — which is therefore load-bearing, not
        // theoretical: without it Tab could never reach Save.
        assert_eq!(order[drawn.len()..], [Field::Save, Field::Cancel]);
    }
}

#[test]
fn every_field_is_reachable_by_tab_at_every_width() {
    // A field Tab cannot reach is a field that cannot be changed.
    for cols in [40, 60, 80, 100, 160, 240] {
        let order = tab_order(cols);
        for f in FIELDS {
            assert!(order.contains(&f), "{f:?} unreachable at {cols} cols");
        }
        let mut seen = order.clone();
        seen.sort_by_key(|f| format!("{f:?}"));
        seen.dedup();
        assert_eq!(seen.len(), order.len(), "a field is tabbed twice at {cols}");
    }
}

#[test]
fn tab_walks_each_card_down_before_moving_to_the_next() {
    // Above STACK_BELOW the form is TWO COLUMNS of cards, so "top to bottom"
    // means down a card and on to the next — not across the two columns, row
    // by row, which is what sorting the rects by y would have given.
    let order = tab_order(160);
    let pos = |f: Field| order.iter().position(|x| *x == f).unwrap();
    // The whole appearance card comes before anything in the window card.
    assert!(pos(Field::Gradient) < pos(Field::NavWidth), "{order:?}");
    // And within a card, the eye's order: font family, then size beside grain.
    assert!(pos(Field::FontFamily) < pos(Field::FontSize));
    assert!(pos(Field::FontSize) < pos(Field::PaperGrain));
    assert!(pos(Field::PaperGrain) < pos(Field::Smooth));
    // Save and Cancel are last, as the last thing you reach.
    assert_eq!(order[order.len() - 2..], [Field::Save, Field::Cancel]);
}
