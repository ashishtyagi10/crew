//! On main the pill only ever changed its number: no frame marked a new
//! arrival, and the busy branch never heard of it.
use super::*;
use crate::motion::{set_level, MotionLevel};

/// The pill inverts (accent block, page ink) on each increase of N for
/// POP_MS, then is the resting accent-on-page again; Off never pops.
#[test]
fn the_pill_pops_on_each_increase_then_rests() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let accent = crate::palette::accent();
    let page = crew_theme::theme().page_bg;
    let pop = Pop::default();
    assert!(pop.observe(1, 1_000), "the first unread is an increase");
    assert!(pop.live(1_000 + POP_MS - 1));
    assert!(
        !pop.observe(1, 1_000 + POP_MS),
        "landed: the same N is quiet"
    );
    assert!(pop.observe(2, 2_000), "N grew again");
    assert!(
        !pop.observe(1, 2_000 + POP_MS),
        "a decrease is just recorded"
    );
    pop.reset();
    assert!(pop.observe(1, 3_000), "after the bottom, 1 is new again");

    let popped = crate::chatscroll::pill_cells(3, 80, 5, true);
    let rest = crate::chatscroll::pill_cells(3, 80, 5, false);
    assert_eq!(popped.len(), rest.len());
    assert!(popped.iter().all(|c| c.bg == accent && c.bold), "inverted");
    assert!(
        popped
            .iter()
            .all(|c| crew_theme::contrast_ratio(c.fg, c.bg) >= crew_theme::contrast::text_floor()),
        "the ink on the block clears the text floor"
    );
    assert!(rest
        .iter()
        .all(|c| c.bg == page && c.fg == accent && c.bold));

    set_level(MotionLevel::Off);
    let pop = Pop::default();
    assert!(!pop.observe(1, 1_000), "Off: no pop");
    set_level(MotionLevel::Full);
}
