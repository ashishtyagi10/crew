use ratatui::layout::{Position, Rect};

use super::form::{dim, ink, input_box, layout, scroll_for};
use super::{Field, FIELDS};
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

#[test]
fn a_wide_pane_deals_the_cards_into_balanced_columns() {
    let lay = layout(160);
    let mut xs: Vec<u16> = lay.cards.iter().map(|c| c.rect.x).collect();
    xs.sort_unstable();
    xs.dedup();
    assert!(xs.len() > 1, "a whole window still drew one column: {xs:?}");
    // Balanced: no column is taller than the shortest one plus the tallest
    // card, which is the most a shortest-column deal can be out by.
    let bottom = |x: u16| -> u16 {
        lay.cards
            .iter()
            .filter(|c| c.rect.x == x)
            .map(|c| c.rect.y + c.rect.height)
            .max()
            .unwrap_or(0)
    };
    let tallest = lay.cards.iter().map(|c| c.rect.height).max().unwrap();
    let (lo, hi) = (
        xs.iter().map(|&x| bottom(x)).min().unwrap(),
        xs.iter().map(|&x| bottom(x)).max().unwrap(),
    );
    assert!(hi <= lo + tallest, "columns {lo} and {hi} are not balanced");
    assert_eq!(lay.height, hi, "the form is as tall as its tallest column");
    // Within a column the cards stack in order and never overlap.
    for x in xs {
        let col: Vec<&super::form::Card> = lay.cards.iter().filter(|c| c.rect.x == x).collect();
        for w in col.windows(2) {
            assert!(
                w[1].rect.y >= w[0].rect.y + w[0].rect.height,
                "{} overlaps {}",
                w[1].title,
                w[0].title
            );
        }
    }
}

/// The point of the split and the deal: a whole window's worth of settings
/// fits a whole window, rather than scrolling one tall column past a column
/// of empty page.
#[test]
fn a_whole_window_does_not_have_to_scroll() {
    let one = layout(40).height;
    let wide = layout(160).height;
    assert!(
        wide * 2 < one,
        "160 columns saved nothing: {wide} vs {one} stacked"
    );
    assert!(wide <= 30, "a whole window still scrolls at {wide} rows");
}

#[test]
fn narrow_pane_stacks_single_column() {
    let lay = layout(48);
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
