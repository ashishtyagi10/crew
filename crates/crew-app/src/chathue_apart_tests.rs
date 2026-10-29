//! No two palettes draw code alike: split from `chathue_tests` (line cap).
use super::super::*;
use crew_theme::oklch::distance;
use crew_theme::RandomMode;

/// The mean OKLab step across the six code roles a pair must clear: the
/// visible step the roster's near-duplicate gate and `ring_tests` use. Until
/// 2026-10-29 `nebula`, `glass-clear` and `glass-night` shared crew's six
/// hues and crew's map, and drew code 0.004–0.009 apart, one look under
/// three names; `blossom` handed out its hues as `amethyst` does, 0.041.
const FLOOR: f32 = 0.05;

/// Two classics answer to their schemes, not to crew: everforest and gruvbox
/// really do key in red, type in yellow and count in purple alike (0.030).
/// They still may not be one look.
const CLASSIC_FLOOR: f32 = 0.025;

fn roles(t: &Theme) -> [Color; 6] {
    let (base, h) = (chatink::derive(t), derive(t));
    [h.keyword, h.ty, h.func, h.number, base.string, base.code]
}

#[test]
fn no_two_palettes_draw_code_alike() {
    let _g = crate::app::theme_test_guard();
    let mut pairs = 0;
    for (n, a) in ALL_THEMES.iter().enumerate() {
        for b in &ALL_THEMES[n + 1..] {
            let (x, y) = (roles(a.theme()), roles(b.theme()));
            let gap = (0..6).map(|k| distance(x[k], y[k])).sum::<f32>() / 6.0;
            let floor = match [a, b].map(|id| id.mode() == RandomMode::Classic) {
                [true, true] => CLASSIC_FLOOR,
                _ => FLOOR,
            };
            assert!(
                gap >= floor,
                "{} and {} draw code {gap:.3} apart (need {floor})",
                a.as_str(),
                b.as_str()
            );
            pairs += 1;
        }
    }
    assert_eq!(pairs, ALL_THEMES.len() * (ALL_THEMES.len() - 1) / 2);
}
