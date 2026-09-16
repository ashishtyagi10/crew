use ratatui::layout::{Position, Rect};

use super::form::{dim, ink, input_box, layout, scroll_for, tab_order, STACK_BELOW};
use super::{Field, FIELDS};
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

#[test]
fn wide_pane_lays_out_two_columns() {
    let lay = layout(80);
    // Structural rather than a card count: appearance owns the left column
    // alone and everything else stacks down the right. Pinning the number
    // meant adding a card broke this test without saying anything about the
    // layout, which is what happened when usage arrived.
    let (appearance, right) = lay.cards.split_first().expect("at least one card");
    assert_eq!(appearance.title, "appearance");
    assert!(right.len() >= 2, "the right column is empty");
    for c in right {
        assert!(
            c.rect.x > appearance.rect.x,
            "{} is not in the right column",
            c.title
        );
        assert_eq!(c.rect.x, right[0].rect.x, "{} broke the column", c.title);
    }
    for w in right.windows(2) {
        assert!(
            w[1].rect.y >= w[0].rect.y + w[0].rect.height,
            "{} overlaps {}",
            w[1].title,
            w[0].title
        );
    }
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

/// The card legends are lowercase, like every legend on the canvas — the
/// sidebar's capitals are the sidebar's idiom.
#[test]
fn card_legends_are_lowercase_like_the_canvas() {
    for c in layout(80).cards.iter().chain(layout(40).cards.iter()) {
        assert_eq!(c.title, c.title.to_lowercase(), "{:?}", c.title);
    }
}

/// The focused box's legend is bold as well as accent; an unfocused one is
/// neither. And an empty box shows its hint muted until it is typed into.
#[test]
fn the_focused_box_is_bold_and_an_empty_box_says_what_empty_means() {
    let _g = crate::app::theme_test_guard();
    let r = Rect::new(0, 0, 20, 3);
    let legend_bold = |focused: bool| {
        let mut buf = Buffer::empty(r);
        input_box(
            &mut buf,
            r,
            "Accent",
            "",
            focused,
            true,
            Some("theme's own"),
        );
        let bold = (0..20)
            .any(|x| buf[(x, 0)].symbol() == "A" && buf[(x, 0)].modifier.contains(Modifier::BOLD));
        let row: String = (1..19).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        (bold, row.trim_end().to_string(), buf[(1, 1)].fg)
    };
    let (bold, row, fg) = legend_bold(false);
    assert!(!bold);
    assert_eq!(row, "theme's own", "the hint fills the empty box");
    assert_eq!(fg, dim(), "muted");
    let (bold, row, _) = legend_bold(true);
    assert!(bold, "focused: bold legend");
    assert_eq!(row, "\u{2588}", "focused: the cursor, not the hint");
    let mut buf = Buffer::empty(r);
    input_box(
        &mut buf,
        r,
        "Accent",
        "#ff0000",
        false,
        true,
        Some("theme's own"),
    );
    assert_eq!(buf[(1, 1)].fg, ink(), "a typed value is ink, hint gone");
}

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
