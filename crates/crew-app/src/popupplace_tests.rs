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
