use glyphon::ColorMode;

use super::atlas_color_mode;

#[test]
fn atlas_color_mode_matches_the_target_kind() {
    // Non-sRGB target → Web (gamma-space blending, sRGB values pass through).
    // sRGB-only platform → Accurate (values linearized; never wash out).
    assert_eq!(atlas_color_mode(false), ColorMode::Web);
    assert_eq!(atlas_color_mode(true), ColorMode::Accurate);
}
