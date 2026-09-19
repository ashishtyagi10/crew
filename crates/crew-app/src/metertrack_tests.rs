use super::*;

/// Both meter families' tracks read on every page in the set. They were
/// measured as COLOURS before, and passed: the capsule lays a track down at
/// 55% alpha, and that is where the contrast went.
#[test]
fn every_track_survives_being_drawn_on_every_page() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let mut thin: Vec<String> = Vec::new();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let page = crew_theme::theme().page_bg;
        for (what, c) in [
            ("footer", crate::summarymeter::meter_track()),
            ("rail", crate::gauges::track_color()),
        ] {
            let r = crew_theme::contrast_ratio(as_drawn(c, page), page);
            if r < FLOOR - 0.01 {
                thin.push(format!("{id:?} {what}: {r:.2}"));
            }
        }
    }
    assert!(
        thin.is_empty(),
        "tracks that vanish into the page: {thin:?}"
    );
}

/// The lift stops as soon as it clears the floor: a track is recessed on
/// purpose, and one walked all the way to the ink would read as a full bar.
#[test]
fn the_lift_takes_the_least_it_can() {
    let _g = crate::app::theme_test_guard();
    let page = (0, 0, 0);
    let from = (10, 10, 10);
    let to = (255, 255, 255);
    let c = lift(from, to, page);
    assert!(c != to, "walked the whole way: {c:?}");
    let r = crew_theme::contrast_ratio(as_drawn(c, page), page);
    assert!(r >= FLOOR - 0.01, "{r:.2} is under the floor");
    assert!(r < FLOOR + 0.35, "{r:.2} overshot the floor");
}

/// A colour that already reads is left exactly as it is.
#[test]
fn a_track_that_already_reads_is_untouched() {
    let _g = crate::app::theme_test_guard();
    let page = (0, 0, 0);
    let from = (200, 200, 200);
    assert_eq!(lift(from, (255, 255, 255), page), from);
}
