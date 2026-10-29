//! A tube's syntax ladder (`chathue::tube`): one hue, so brightness is the
//! whole of the highlighting, and every rung has to be a visible step.
use super::super::*;

/// How far apart two neighbouring rungs must be, as a contrast ratio. The
/// ladder spaces them about 1.2; 1.15 is a brightness step you can see
/// between two words side by side on a phosphor.
const RUNG: f32 = 1.15;

/// Prose, keyword, identifier, plain code (and numbers), string, comment:
/// each a rung brighter than the next, on every tube. Before the ladder,
/// keyword, type, call, number and code measured one 7.4:1 on crt-green.
#[test]
fn a_tube_draws_its_syntax_on_a_brightness_ladder() {
    let _g = crate::app::theme_test_guard();
    let mut tubes = 0;
    for id in ALL_THEMES.into_iter().filter(|id| id.theme().is_tube()) {
        tubes += 1;
        let t = id.theme();
        let (base, h) = (chatink::derive(t), derive(t));
        let ladder = [
            ("prose", t.ink),
            ("keyword", h.keyword),
            ("identifier", h.func),
            ("code", base.code),
            ("string", base.string),
            ("comment", base.comment),
        ];
        for w in ladder.windows(2) {
            let (hi, lo) = (w[0], w[1]);
            let step = contrast_ratio(hi.1, t.page_bg) / contrast_ratio(lo.1, t.page_bg);
            assert!(
                step >= RUNG,
                "{}: {} only {step:.3}x over {}",
                id.as_str(),
                hi.0,
                lo.0
            );
        }
    }
    assert_eq!(tubes, 2);
}

/// Every rung keeps the phosphor: nothing on the ladder strays off the
/// ink's hue, which is the whole of a tube's identity.
#[test]
fn every_rung_keeps_the_phosphors_hue() {
    let _g = crate::app::theme_test_guard();
    let hue = |c| crew_theme::oklch::from_srgb(c).h;
    let gap = |a: f32, b: f32| (a - b).rem_euclid(360.0).min((b - a).rem_euclid(360.0));
    for id in ALL_THEMES.into_iter().filter(|id| id.theme().is_tube()) {
        let t = id.theme();
        let h = derive(t);
        for (what, c) in [("keyword", h.keyword), ("identifier", h.func)] {
            let d = gap(hue(c), hue(t.ink));
            assert!(d < 6.0, "{}: {what} {d:.1}° off the ink", id.as_str());
        }
    }
}
