use super::inked;
use crew_theme::{contrast_ratio, oklch, slot_color, ThemeId};

/// Neon's badges (an agent's name, a pane's legend) are neons: every
/// signature slot's fill is as vivid as sRGB holds at its depth, and white
/// still clears the floor on it. Deepened at the pale tag's own chroma,
/// smith's fill was 0.07, half of what its depth allowed.
#[test]
fn neon_badges_are_vivid_and_still_read_in_white() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::NeonDark);
    let t = crew_theme::theme();
    let floor = crew_theme::contrast::text_floor();
    for slot in 0..12 {
        let (fg, bg) = inked(slot_color(slot, t));
        let o = oklch::from_srgb(bg);
        let most = oklch::from_srgb(oklch::Oklch::new(o.l, 0.4, o.h).to_srgb()).c;
        assert!(
            o.c >= 0.9 * most,
            "slot {slot}: {bg:?} chroma {:.3} of {most:.3}",
            o.c
        );
        let cr = contrast_ratio(fg, bg);
        assert!(cr >= floor, "slot {slot}: white on {bg:?} {cr:.2}");
    }
}

/// Glass that is not holographic keeps its fills as they were.
#[test]
fn plain_glass_fills_keep_their_own_chroma() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::GlassNight);
    let t = crew_theme::theme();
    for slot in 0..12 {
        let tag = slot_color(slot, t);
        let (_, bg) = inked(tag);
        let (was, now) = (oklch::from_srgb(tag).c, oklch::from_srgb(bg).c);
        assert!(now <= was + 0.02, "slot {slot}: {was:.3} → {now:.3}");
    }
}
