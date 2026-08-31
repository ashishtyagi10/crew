use crew_theme::{contrast_ratio, ALL_THEMES};

/// Every description in every picker is drawn in this one colour, so a
/// preset where it does not read takes five callers down at once.
#[test]
fn the_description_ink_reads_on_every_preset() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let got = contrast_ratio(super::desc(), t.page_bg);
        assert!(
            got >= crew_theme::contrast::mark_floor(),
            "{}: menu description vs page = {got:.2}",
            id.as_str()
        );
    }
}

/// A single-phosphor tube can draw ONE hue. The constant this replaced
/// was `(120, 130, 140)` — a blue-grey, which is a colour a green screen
/// does not have; it cleared every contrast floor while being visibly the
/// wrong ink. Contrast was never the defect, so contrast alone cannot be
/// the guard: the description has to be the theme's own muted colour.
#[test]
fn on_a_single_phosphor_tube_the_description_is_on_the_phosphor() {
    let _g = crate::app::theme_test_guard();
    let mut checked = 0;
    for id in ALL_THEMES {
        crew_theme::set_theme(id);
        // `is_tube` is the theme's OWN single-phosphor predicate (a
        // `CrtStyle` that actually scans). Every preset now carries a
        // `ModernStyle` as a bloom vehicle, so the older
        // `crt.is_some() && modern.is_none()` spelling of this excludes
        // all twelve — see the counter below.
        if !id.is_crt() {
            continue;
        }
        let t = crew_theme::theme();
        let (want, got) = (
            crew_theme::oklch::from_srgb(t.ink),
            crew_theme::oklch::from_srgb(super::desc()),
        );
        let off = (want.h - got.h).abs().min(360.0 - (want.h - got.h).abs());
        assert!(
            off <= 25.0,
            "{}: description hue {:.0} is {off:.0} deg off the phosphor's {:.0}",
            id.as_str(),
            got.h,
            want.h,
        );
        // ...and it is actually LIT, not a grey that happens to land near
        // the right hue. A grey clears every contrast floor while being
        // the one ink a single-phosphor tube cannot make.
        assert!(
            got.c >= want.c * 0.25,
            "{}: description chroma {:.3} is a grey beside the phosphor's {:.3}",
            id.as_str(),
            got.c,
            want.c,
        );
        checked += 1;
    }
    // The tubes are the whole point of this test; a loop that skipped
    // every one of them is a test that asserts nothing.
    assert_eq!(checked, 4, "every CRT preset was checked");
}

/// …and it stays quieter than the label beside it, or it is not a
/// description any more.
#[test]
fn the_description_stays_under_body_text() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let desc = contrast_ratio(super::desc(), t.page_bg);
        let ink = contrast_ratio(t.ink, t.page_bg);
        assert!(
            desc <= ink,
            "{}: {desc:.2} is not under {ink:.2}",
            id.as_str()
        );
    }
}
