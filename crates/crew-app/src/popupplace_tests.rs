use super::above_composer;
use crate::chat::ChatPane;
use crate::layout::Rect;
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only layout is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

/// The pop-up's bottom edge is the composer's top edge — with the summary
/// footer's rows UNDER the composer accounted for. Subtracting the
/// composer alone (the old placement) put the pop-up `summary` rows too
/// low: over the composer, hiding what was being typed.
#[test]
fn a_popup_stands_on_the_composer_not_on_the_footer() {
    let mut p = pane();
    p.input = "/model claude".into();
    let (cw, ch) = (8.0, 16.0);
    let r = Rect {
        x: 10.0,
        y: 20.0,
        w: 100.0 * cw,
        h: 40.0 * ch,
    };
    let (cols, rows) = (100, 40);
    let g = crate::chatplace::grants(&p, cols, rows);
    assert!(
        g.summary > 0,
        "a 40-row pane has a footer: summary={}",
        g.summary
    );
    assert!(g.bottom > g.summary, "and a composer above it");
    let mh = 12.0 * ch;
    let y = above_composer(&p, r, cw, ch, mh);
    let composer_top = r.y + f32::from(rows - g.bottom) * ch;
    assert_eq!(y + mh, composer_top);
    // The old formula lands on the composer, `summary` rows below.
    let old = r.y + r.h - f32::from(g.bottom - g.summary) * ch - mh;
    assert!(old + mh > composer_top, "old={old} top={composer_top}");
}

/// A pop-up taller than the room above is pinned to the pane's top, never
/// above it.
#[test]
fn a_tall_popup_is_pinned_to_the_pane_top() {
    let p = pane();
    let r = Rect {
        x: 0.0,
        y: 30.0,
        w: 800.0,
        h: 8.0 * 16.0,
    };
    assert_eq!(above_composer(&p, r, 8.0, 16.0, 20.0 * 16.0), 30.0);
}

/// The card is as wide as its rows plus the frame — floored so a one-word
/// list is still a card, and never wider than the pane it stands in.
#[test]
fn card_cols_hugs_the_content_floored_and_clamped() {
    use super::{card_cols, MIN_COLS};
    assert_eq!(card_cols(10, 100), MIN_COLS, "floored");
    assert_eq!(card_cols(60, 100), 62, "content plus two border columns");
    assert_eq!(card_cols(200, 100), 100, "clamped to the pane");
    assert_eq!(card_cols(usize::MAX, 100), 100, "no overflow");
}

/// The scene stands on the composer, flush with the pane's left edge, and
/// is exactly as wide as the card — not the pane.
#[test]
fn a_popup_scene_is_as_wide_as_its_card_and_flush_left() {
    let p = pane();
    let (cw, ch) = (8.0, 16.0);
    let r = Rect {
        x: 10.0,
        y: 20.0,
        w: 100.0 * cw,
        h: 40.0 * ch,
    };
    let popup = super::Popup {
        cells: Vec::new(),
        cols: 40,
        rows: 5,
    };
    let s = super::scene(&p, r, cw, ch, popup);
    // The card, plus one column of page as a margin against the text under it.
    assert_eq!((s.x, s.w, s.h), (r.x, 41.0 * cw, 5.0 * ch));
    assert!(s.overlay, "held solid by the overlay pass");
    let g = crate::chatplace::grants(&p, 100, 40);
    assert_eq!(s.y + s.h, r.y + f32::from(40 - g.bottom) * ch);
}

/// The margin never pushes the scene past the pane: a card as wide as the
/// pane gets no margin, and the scene is never narrower than the card.
#[test]
fn the_margin_stops_at_the_pane_edge() {
    use super::scene_w;
    assert_eq!(scene_w(40, 100, 8.0), 41.0 * 8.0);
    assert_eq!(scene_w(100, 100, 8.0), 100.0 * 8.0, "no room for a margin");
    assert_eq!(
        scene_w(60, 50, 8.0),
        60.0 * 8.0,
        "never narrower than the card"
    );
}
