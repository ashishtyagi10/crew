use super::smoke;

const SMOKE: (u8, u8, u8) = (17, 18, 22);
const INK: (u8, u8, u8) = (238, 239, 241);

fn lerp(t: f32) -> (u8, u8, u8) {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    (m(SMOKE.0, INK.0), m(SMOKE.1, INK.1), m(SMOKE.2, INK.2))
}

/// The code field — the smoke walked toward the ink — is glass; a hue, the
/// page itself, the ink itself and anything past either end stay solid.
#[test]
fn only_a_step_from_the_page_toward_the_ink_is_glass() {
    for t in [0.05, 0.18, 0.4, 0.9] {
        assert!(smoke(lerp(t), SMOKE, INK), "{t}");
    }
    for c in [
        (54, 84, 130),
        SMOKE,
        INK,
        (255, 255, 255),
        (5, 5, 8),
        (120, 40, 40),
    ] {
        assert!(!smoke(c, SMOKE, INK), "{c:?}");
    }
    assert!(!smoke((57, 58, 61), SMOKE, SMOKE), "no line");
}

/// Off glass, or on an opaque window, every background is solid.
#[test]
fn a_solid_window_draws_every_background_solid() {
    let t = crew_theme::theme();
    let field = (
        t.page_bg.0 / 2 + t.ink.0 / 2,
        t.page_bg.1 / 2 + t.ink.1 / 2,
        t.page_bg.2 / 2 + t.ink.2 / 2,
    );
    assert_eq!(super::alpha(field, false), 1.0);
}
